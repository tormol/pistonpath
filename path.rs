/* Copyright (C) 2015 by Alexandru Cojocaru */

/* This program is free software: you can redistribute it and/or modify
   it under the terms of the GNU General Public License as published by
   the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU General Public License for more details.

   You should have received a copy of the GNU General Public License
   along with this program.  If not, see <http://www.gnu.org/licenses/>. */

#![allow(non_snake_case)]
const FONT_PATH: &'static str = "/usr/share/fonts/truetype/msttcorefonts/arial.ttf";
const FONT_RESOLUTION: f64 = 100.0;
const BORDER_RADIUS: f64 = 0.03;//where 1 is tile_size
const TILE_MIN_PADDING: f64 = 0.1;
const SHOW_DIGITS: f64 = 3.0;
const INITIAL_TILE_SIZE: f64 = 50.0;

const BOARD_WIDTH: i32 = 20;
const BOARD_HEIGHT: i32 = 15;
const UPDATE_TIME: f64 = 0.15;

extern crate vecmath;
use vecmath::Vector2;
extern crate graphics;
use graphics::math::{Vec2d, Matrix2d, Scalar};

type Point = Vector2<i32>;// Vector2 is [T; 2]
trait Intpointadd {
    fn plus(&self, other:Point) -> Point;//coherence rules means we cant implement Add :|
}
impl Intpointadd for Point {
    fn plus(&self, other:Point) -> Point {
        [self.x()+other.x(), self.y()+other.y()]
    }
}
trait Point2<T> {
    fn x(&self) -> T;
    fn y(&self) -> T;
}
impl Point2<i32> for Point {
    fn x(&self) -> i32 {self[0]}
    fn y(&self) -> i32 {self[1]}
}

impl Point2<Scalar> for Vec2d {
    fn x(&self) -> Scalar {self[0]}
    fn y(&self) -> Scalar {self[1]}
}


#[derive(PartialEq, Copy, Clone)]
enum Direction {North, South, East, West,}
impl Direction {
    fn unit_vector(&self) -> Point { match *self {
        Direction::North => [ 0,  1],
        Direction::South => [ 0, -1],
        Direction::East  => [ 1,  0],
        Direction::West  => [-1,  0],
    }}
}

#[derive(PartialEq, Copy, Clone)]
struct Path {
    distance : i32,
    next : Direction,
}

#[derive(PartialEq, Copy, Clone)]
enum Tile {
    Wall,
    Target,
    Open (Option<Path>)
}
use self::Tile::*;//use Wall instead of Tile::Wall
impl Tile {
    fn color(&self) -> Color { match *self {
        Wall    => color::hex("002951"),
        Target  => color::hex("8ba673"),
        Open(_) => color::hex("001122"),
    }}
}

use graphics::{Context,DrawState,Transformed,color,math};
use graphics::types::Color;
use std::cmp;

extern crate piston;
use piston::input::keyboard::Key;
use piston::input::mouse::MouseButton;

extern crate opengl_graphics;
use opengl_graphics::GlGraphics;
use opengl_graphics::glyph_cache::GlyphCache;

type Board = [[Tile; BOARD_WIDTH as usize]; BOARD_HEIGHT as usize];
struct Game {
    board: Board,
    mover: Point,
    target: Option<Point>,
    mouse_pos: Option<Point>,
    selection_start: Option<Point>,
    paused: bool,
    time: f64,
    update_time: f64,
    //static resources
    res_character_cache: GlyphCache<'static>,
} impl Game {
    fn new() -> Game {
        Game {
            res_character_cache: GlyphCache::new(std::path::Path::new(FONT_PATH)).unwrap(),
            time: 0.0,
            update_time: 0.0,
            paused: false,
            selection_start: None,
            mouse_pos: None,
            target: None,
            mover: [1,1],
            board: [[Tile::Open(None); BOARD_WIDTH as usize]; BOARD_HEIGHT as usize],
        }
    }

    //in the returned pair, first,x<=second.x and first.y<=second.y, now they can be uused in a loop or draw
    fn order_points(a:Point, b:Point) -> (Point,Point) {
        ([cmp::min(a.x(), b.x()),  cmp::min(a.y(), b.y())],
         [cmp::max(a.x(), b.x()),  cmp::max(a.y(), b.y())])
    }

    fn render(&mut self,  draw_state: DrawState,  transform: math::Matrix2d, gfx: &mut GlGraphics) {
        //use graphics::rectangle;
        extern crate num;
        fn to_f64_4<T: num::ToPrimitive>(a:T, b:T, c:T, d:T) -> [f64; 4] {
            [a.to_f64().unwrap(), b.to_f64().unwrap(), c.to_f64().unwrap(), d.to_f64().unwrap()]
        }

        graphics::clear(color::BLACK, gfx);//comment out and see!

        //tiles
        for (y_usize,ref row) in self.board.into_iter().enumerate() {
            for (x_usize,tile) in row.into_iter().enumerate() {
                let (x,y) = (x_usize as f64, y_usize as f64);
                graphics::rectangle(tile.color(), [x,y,1.0,1.0], transform, gfx);
                if let Open(Some(path)) = *tile {
                    let as_str: &str = &path.distance.to_string()[..];//[..] converts String to str
                    let digits = as_str.len() as f64;//digits aren't unicode
                    let digit_height = 0.8;
                    let digit_width = 0.6;
                    let width = digit_width*SHOW_DIGITS;
                    let scale_fill = f64::max(digit_height, width) - TILE_MIN_PADDING - BORDER_RADIUS;
                    let bottom_padding = (1.0-digit_height/scale_fill) / 2.0;
                    let left_padding = (1.0-width/scale_fill) / 2.0  +  (digit_width/(scale_fill-TILE_MIN_PADDING)) * (SHOW_DIGITS-digits);
                    let scale_factor = 1.0 / (scale_fill*FONT_RESOLUTION);
                    let char_pos = transform
                        .trans(x + left_padding,  1.0 + y - bottom_padding)
                        .scale(scale_factor, scale_factor);
                    graphics::text::Text::new_color(Target.color(), FONT_RESOLUTION as u32)
                        .draw(as_str, &mut self.res_character_cache, &draw_state, char_pos, gfx);
                }
                if self.mover.x() == x_usize as i32
                && self.mover.y() == y_usize as i32 {
                    let red = color::hex("ee2222");
                    let posdim = [x+0.3,y+0.3,0.4,0.4];
                    graphics::rectangle(red, posdim, transform, gfx);
                }
            }
        }

        if let Some(mouse_pos) = self.mouse_pos {
            //selection
            if let Some(start) = self.selection_start {
                let (a,b) = Game::order_points(start, mouse_pos);
                let rect = to_f64_4(a.x(), a.y(),  b.x()-a.x()+1, b.y()-a.y()+1);
                let selection_color = [1.0, 1.0, 1.0, 0.2];//white
                graphics::rectangle(selection_color, rect, transform, gfx);
            }
            //hover
            let mouse_color = [0.9, 1.0, 0.9, 0.1];//light green
            graphics::rectangle(mouse_color,  to_f64_4(mouse_pos.x(), mouse_pos.y(), 1, 1),  transform,  gfx);
        }

        //border lines
        let line_color = [0.4, 0.4, 0.4, 0.3];//grey
        for y in 1..BOARD_HEIGHT {
            graphics::line(line_color, BORDER_RADIUS, to_f64_4(0,y,BOARD_WIDTH,y),  transform, gfx);
        }
        for x in 1..BOARD_WIDTH {
            graphics::line(line_color, BORDER_RADIUS, to_f64_4(x,0,x,BOARD_HEIGHT),  transform, gfx);
        }
    }

    fn update(&mut self, dt: f64) {
        if self.paused {
            return;
        }
        self.update_time += dt;
        if self.update_time-self.time < UPDATE_TIME {
            return;
        }
        self.time = self.update_time;
        let m = self.mover;
        if let Open(Some(path)) = self.board[m.y() as usize][m.x() as usize] {
            self.mover = m.plus(path.next.unit_vector());
        }
    }

    fn update_paths(&mut self) {
        //reset all
        for row in &mut self.board {
            for tile in &mut row.iter_mut() {
                if let Open(Some(_)) = *tile {
                    *tile = Open(None);
        }   }   }

        if let Some(target) = self.target {
            use std::collections::vec_deque::VecDeque;
            use Direction::*;

            fn go<'a>(board: &'a mut Board,  p: Point,  from_dist: i32,  from_dir: Direction) -> bool {
                if p.x()>=0  &&  p.x()<BOARD_WIDTH
                && p.y()>=0  &&  p.y()<BOARD_HEIGHT {
                    let tile = &mut board[p.y()as usize][p.x()as usize];
                    if let Open(to_path) = *tile {
                        let default_path = Path{distance: std::i32::MAX,  next: North};
                        if from_dist < to_path.unwrap_or(default_path).distance {
                            *tile = Open(Some(Path{distance: from_dist,  next: from_dir}));
                            true
                        } else {false}
                    } else if from_dist==0 && *tile == Target {
                        true//initial tile
                    } else {false}
                } else {false}
            }

            let mut to_check : VecDeque<(Point, i32, Direction)> = VecDeque::new();
            to_check.push_back((target, 0, South));
            while let Some((from_pos, from_dist, from_dir)) = to_check.pop_front() {
                if go(&mut self.board,  from_pos,  from_dist, from_dir) {
                    to_check.push_back((from_pos.plus(North.unit_vector()), from_dist+1, South));
                    to_check.push_back((from_pos.plus(South.unit_vector()), from_dist+1, North));
                    to_check.push_back((from_pos.plus( West.unit_vector()), from_dist+1,  East));
                    to_check.push_back((from_pos.plus( East.unit_vector()), from_dist+1,  West));
                }
            }
        }
    }

    fn mouse_move(&mut self,  pos: Option<Point>) {
        self.mouse_pos = pos;
        if pos.is_none() {
            self.selection_start = None;
        }
    }
    fn mouse_press(&mut self,  button: MouseButton) {
        if button == MouseButton::Left  &&  self.mouse_pos.is_some() {
            self.selection_start = self.mouse_pos;
        }
    }

    fn mouse_release(&mut self,  button: MouseButton) {
        match (button, self.mouse_pos) {
            (MouseButton::Left, Some(end)) => {
                if let Some(start) = self.selection_start {
                    self.selection_start = None;

                    let from = self.board[start.y() as usize][start.x() as usize];
                    let set = match from {Open(_)=>{Wall} Wall=>{Open(None)} Target=>{return}};

                    let (first, second) = Game::order_points(start, end);
                    for row in &mut self.board[first.y()as usize .. 1+second.y()as usize] {
                        for tile in &mut row[first.x()as usize .. 1+second.x()as usize].iter_mut() {
                            if *tile != Target {
                                *tile = set;
            }   }   }   }   }
            (MouseButton::Right, Some(pos))  =>  {
                let mut set = true;
                if let Some(target) = self.target {
                    self.board[target.y() as usize][target.x() as usize] = Open(None);
                    self.target = None;
                    set = pos != target;
                }
                if set {
                    self.board[pos.y() as usize][pos.x() as usize] = Target;
                    self.target = Some(pos);
                }
            }
            (_,_) => {}
        }
        self.update_paths();
    }

    fn key_press(&mut self,  key: Key) {
        if key == Key::P {
            self.paused != self.paused;
        }
    }
}

use piston::window::WindowSettings;
use piston::event_loop::{Events,WindowEvents};
use piston::input::{Button, Motion, Event, Input, RenderEvent};
use opengl_graphics::OpenGL;
use graphics::draw_state::Blend;

extern crate piston_window;
use piston_window::PistonWindow;

fn main() {
    println!("P => Pause");

    let mut window: PistonWindow =
        WindowSettings::new("PistonPath", [
                INITIAL_TILE_SIZE as u32  *  BOARD_WIDTH as u32,
                INITIAL_TILE_SIZE as u32  *  BOARD_HEIGHT as u32
            ]).exit_on_esc(true).build().unwrap();

    let mut gfx = GlGraphics::new(OpenGL::V3_2);
    //by default alpha blending is disabled, which means all semi-transparent colors are considered opaque.
    //since colors are blended pixel for pixel, this has a performance cost,
    //the alternative is to check for existing color in tile, and blend manually, or even statically
    // gfx.enable_alpha_blend();

    let mut tile_size = INITIAL_TILE_SIZE;//changes if window is resized

    let mut game = Game::new();
    let mut event_loop: WindowEvents = window.events();
    while let Some(e) = event_loop.next(&mut window) {
        match e {
            Event::Render(render_args/*: RenderArgs*/) => {
                //update if window has been resized, else weird things would happen
                //THANK YOU Arcterus/game-of-life/src/app.rs
                &mut gfx.viewport(0, 0, render_args.width as i32, render_args.height as i32);
                //TODO: center letterboxing

                let context: Context = Context::new_viewport(render_args.viewport()).scale(tile_size, tile_size);
                let transform: Matrix2d = context.transform;
                context.draw_state.blend(Blend::Alpha);

                game.render(context.draw_state, transform, &mut gfx);
            }
            Event::Update(update_args) => {
                game.update(update_args.dt);//deltatime is its only field
            }

            Event::Input(Input::Press(Button::Keyboard(key))) => {
                game.key_press(key);
            }
            Event::Input(Input::Press(Button::Mouse(button))) => {
                game.mouse_press(button);
            }
            Event::Input(Input::Release(Button::Mouse(button))) => {
                game.mouse_release(button);
            }
            Event::Input(Input::Resize(x,y)) => {
                tile_size = f64::min( x as f64 / (BOARD_WIDTH as f64),  y as f64 / (BOARD_HEIGHT as f64));
            }
            Event::Input(Input::Move(Motion::MouseCursor(x,y))) => {
                let tile = [(x/ tile_size) as i32, (y/ tile_size) as i32];
                let mut pos = None;
                if tile.x() >= 0  &&  tile.x() < BOARD_WIDTH as i32
                && tile.y() >= 0  &&  tile.y() < BOARD_HEIGHT as i32 {
                    pos = Some(tile)
                }
                game.mouse_move(pos);
            }
            Event::Input(Input::Cursor(_)) => {//only happens if a button is pressed
                game.mouse_move(None);
            }
            _ => {}
        }
    }
}
