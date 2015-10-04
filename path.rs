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

const BOARD_WIDTH: usize = 15;
const BOARD_HEIGHT: usize = 15;
const TILE_SIZE: f64 = 50.0;
const BORDER_RADIUS: f64 = 0.5;
const UPDATE_TIME: f64 = 0.15;


extern crate vecmath;
use vecmath::Vector2;
extern crate graphics;
use graphics::math::{Vec2d, Matrix2d, Scalar};

type Point = Vector2<i32>;
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
    distance : u16,
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

use graphics::{Context,color,math};
use graphics::types::Color;
use std::cmp;

extern crate piston;
use piston::input::keyboard::Key;
use piston::input::mouse::MouseButton;

extern crate opengl_graphics;
use opengl_graphics::GlGraphics;

struct Game {
    board : [[Tile; BOARD_WIDTH]; BOARD_HEIGHT],
    target : Option<*mut Tile>,//should have been an Option<&mut tile>, but rustc complains about lifetimes
    mouse_pos : Option<Point>,
    selection_start : Option<Point>,
    paused : bool,
    time: f64,
    update_time: f64,
} impl Game {
    fn new() -> Game {
        Game {
            time: UPDATE_TIME,
            update_time: UPDATE_TIME,
            paused: false,
            selection_start: None,
            mouse_pos: None,
            target: None,
            board: [[Tile::Open(None); BOARD_WIDTH]; BOARD_HEIGHT],
        }
    }

    //in the returned pair, first,x<=second.x and first.y<=second.y, now they can be uused in a loop or draw
    fn order_points(a:Point, b:Point) -> (Point,Point) {
        ([cmp::min(a.x(), b.x()),  cmp::min(a.y(), b.y())],
         [cmp::max(a.x(), b.x()),  cmp::max(a.y(), b.y())])
    }

    fn render(&mut self,  tile_size: f64,  transform: math::Matrix2d, gfx: &mut GlGraphics) {
        extern crate glutin_window;
        use graphics::{clear, rectangle, Rectangle, Line};
        extern crate num;
        //use ToPrimitive;
        fn mul<T: num::ToPrimitive>(a:T, b:T, c:T, d:T, tile_size:f64) -> [f64; 4] {
            [a.to_f64().unwrap()*tile_size,  b.to_f64().unwrap()*tile_size,  c.to_f64().unwrap()*tile_size,  d.to_f64().unwrap()*tile_size]
        }
        // let mul: FnOnce(T,T,T,T)->f64 = |a:T, b:usize, c:usize, d:usize| -> [f64;4] {//exploiting that Rectangle and line both are [f64; 4]
        //     [a as f64*tile_size,  b as f64*tile_size,  c as f64*tile_size,  d as f64*tile_size]
        // };

        clear(color::hex("000000"), gfx);

        //tiles
        for (y,ref row) in self.board.into_iter().enumerate() {
            for (x,tile) in row.into_iter().enumerate() {
                graphics::rectangle(tile.color(), mul(x,y,1,1, tile_size), transform, gfx);
            }
        }

        if let Some(mouse_pos) = self.mouse_pos {
            //selection
            if let Some(start) = self.selection_start {
                let (a,b) = Game::order_points(start, mouse_pos);
                let rect = mul(a.x(), a.y(),  b.x()-a.x()+1, b.y()-a.y()+1, tile_size);
                let selection_color = [1.0, 1.0, 1.0, 0.2];//white
                graphics::rectangle(selection_color, rect, transform, gfx);
            }
            //hover
            let mouse_color = [0.9, 1.0, 0.9, 0.1];//light green
            graphics::rectangle(mouse_color,  mul(mouse_pos.x(), mouse_pos.y(), 1, 1, tile_size),  transform,  gfx);
        }

        //border lines
        let line_color = [0.4, 0.4, 0.4, 0.8];//grey
        for y in 1..BOARD_HEIGHT {
            graphics::line(line_color, BORDER_RADIUS, mul(0,y,BOARD_WIDTH,y, tile_size),  transform, gfx);
        }
        for x in 1..BOARD_WIDTH {
            graphics::line(line_color, BORDER_RADIUS, mul(x,0,x,BOARD_HEIGHT, tile_size),  transform, gfx);
        }
    }

    fn update(&mut self, dt: f64) {
        if self.paused {
            return;
        }
        self.time += dt;
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
                //cannot move the next line into a function, because that function would borrow whole self
                let tile = &mut self.board[pos.y() as usize][pos.x() as usize];
                let mut remove = false;
                if let Some(old) = self.target {
                    unsafe {*old = Open(None);}//remove old target
                    if old == tile {//doesn't compile if switched
                        self.target = None;
                        remove = true;
                }   }
                if !remove {
                    *tile = Target;
                    self.target = Some(tile);
                }
            }
            (_,_) => {}
        }
    }

    fn key_press(&mut self,  key: Key) {
        if key == Key::P {
            self.paused != self.paused;
        }
    }
}

use piston::window::WindowSettings;
use piston::event_loop::Events;
use piston::input::{Button, Motion, Event, Input, RenderEvent};
use opengl_graphics::OpenGL;

extern crate piston_window;
use piston_window::PistonWindow;

fn main() {
    println!("P => Pause");

    let window: PistonWindow =
        WindowSettings::new("PistonPath", [
                BOARD_WIDTH as u32 * TILE_SIZE as u32,
                BOARD_HEIGHT as u32 * TILE_SIZE as u32
            ]).exit_on_esc(true).build().unwrap();

    let mut gfx = GlGraphics::new(OpenGL::V3_2);
    //by default alpha blending is disabled, which means all semi-transparent colors are considered opaque.
    //since colors are blended pixel for pixel, this has a performance cost,
    //the alternative is to check for existing color in tile, and blend manually, or even statically
    gfx.enable_alpha_blend();

    let mut tile_size = TILE_SIZE;//changes if window is resized

    let mut game = Game::new();
    for e in window.events() {
        match e {
            Event::Render(render_args/*: RenderArgs*/) => {
                let transform: Matrix2d = Context::new_viewport(render_args.viewport()).transform;
                //update if window has been resized, else weird thing would happen
                //THANK YOU Arcterus/game-of-life/src/app.rs
                &mut gfx.viewport(0, 0, render_args.width as i32, render_args.height as i32);
                //TODO: center letterboxing

                game.render(tile_size, transform, &mut gfx);
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
            Event::Input(Input::Focus(false)) => {//a click outsidde the window
                game.mouse_move(None);//best we can do on detecting mouse leaving window
                //see https://github.com/PistonDevelopers/piston/issues/962
            }
            _ => {}
        }
    }
}
