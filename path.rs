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
const UPDATE_TIME: f64 = 0.20;


use std::ops::Neg;
extern crate num;
use num::{Zero,One,ToPrimitive};
extern crate vecmath;
use vecmath::vec2_add;// Vector2 is [T; 2]
// Why not +?
// Any math library in rust have to make a choice: Either use primitive slices or tuples
// which make constructing and destructuring pain-free, or use std::ops::*.
// Rusts coherence rules prevents them for doing both:
// You cannot implement an external trait for a foreign type.
extern crate graphics;
use graphics::{Context,DrawState,Transformed,color,math};
use graphics::types::Color;


#[derive(PartialEq, Copy, Clone)]
enum Direction {North, South, East, West}
use self::Direction::*;
impl Direction {
    /// Is generic so it can produce both floats and integers
    fn unit_vector<T:Zero+One+Neg<Output=T>> (&self) -> [T; 2] {
        match *self {
            North => [T::zero(),       T::one()      ],
            South => [T::zero(),       T::one().neg()],
            East  => [T::one(),        T::zero()     ],
            West  => [T::one().neg(),  T::zero()     ],
        }
    }
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

use std::cmp;
use std::collections::vec_deque::VecDeque;

extern crate piston;
use piston::input::keyboard::Key;
use piston::input::mouse::MouseButton;
extern crate opengl_graphics;
use opengl_graphics::GlGraphics;
use opengl_graphics::glyph_cache::GlyphCache;
extern crate rand;
use rand::Rng;

type Board = [[Tile; BOARD_WIDTH as usize]; BOARD_HEIGHT as usize];
// Contains the game logic
struct Game {
    board: Board,
    drones: Vec<[f64; 2]>,
    target: Option<[i32; 2]>,
    mouse_pos: Option<[i32; 2]>,
    selection_start: Option<[i32; 2]>,
    paused: bool,
    time: f64,
    update_time: f64,
    //static resources
    res_character_cache: GlyphCache<'static>,
} impl Game {
    fn new() -> Game {
        let mut g = Game {
            res_character_cache: GlyphCache::new(std::path::Path::new(FONT_PATH)).unwrap(),
            time: 0.0,
            update_time: 0.0,
            paused: false,
            selection_start: None,
            mouse_pos: None,
            target: Some([BOARD_WIDTH/2, BOARD_HEIGHT/2]),
            drones: Vec::with_capacity(4),
            board: [[Tile::Open(None); BOARD_WIDTH as usize]; BOARD_HEIGHT as usize],
        };
        // set target position
        g.board[BOARD_HEIGHT as usize/2][BOARD_WIDTH as usize/2] = Target;
        g.update_paths();
        // put a drone in the center of each corner tile
        g.drones.push([0.3, 0.3]);
        g.drones.push([0.3, BOARD_HEIGHT as f64-0.7]);
        g.drones.push([BOARD_WIDTH as f64-0.7, 0.3]);
        g.drones.push([BOARD_WIDTH as f64-0.7, BOARD_HEIGHT as f64-0.7]);
        return g;// Just `g` would do, but to me looks unfinished below `g.something`.
    }            // like I forgot to write the rest of the function.

    /// In the returned pair, first[0]<=second[0] and first[1]<=second[1],
    /// now they can be uused in a loop or draw
    fn order_points(a:[i32; 2], b:[i32; 2]) -> ([i32; 2],[i32; 2]) {
        ([cmp::min(a[0], b[0]),  cmp::min(a[1], b[1])],
         [cmp::max(a[0], b[0]),  cmp::max(a[1], b[1])])
    }

    fn render(&mut self,  draw_state: DrawState,  transform: math::Matrix2d,  gfx: &mut GlGraphics) {
        fn to_f64_4<T: ToPrimitive>(a:T, b:T, c:T, d:T) -> [f64; 4] {
            [a.to_f64().unwrap(), b.to_f64().unwrap(), c.to_f64().unwrap(), d.to_f64().unwrap()]
        }

        graphics::clear(color::BLACK, gfx);//comment out and see!

        //tiles
        for (y_usize,ref row) in self.board.into_iter().enumerate() {
            for (x_usize,tile) in row.into_iter().enumerate() {
                let (x,y) = (x_usize as f64, y_usize as f64);
                graphics::rectangle(tile.color(), [x,y,1.0,1.0], transform, gfx);
                if let Open(Some(path)) = *tile {
                    // number rendering
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
            }
        }

        // drones
        for p in &self.drones {
            let red = color::hex("ee2222");
            let brown = color::hex("330000");
            let border = [p[0],p[1],0.4,0.4];
            let main = [p[0]+0.05,p[1]+0.05,0.3,0.3];
            graphics::rectangle(brown, border, transform, gfx);
            graphics::rectangle(red, main, transform, gfx);
        }

        // hover highlight and selection
        if let Some(mouse_pos) = self.mouse_pos {
            //selection
            if let Some(start) = self.selection_start {
                let (a,b) = Game::order_points(start, mouse_pos);
                let rect = to_f64_4(a[0], a[1],  b[0]-a[0]+1, b[1]-a[1]+1);
                let selection_color = [1.0, 1.0, 1.0, 0.2];//white
                graphics::rectangle(selection_color, rect, transform, gfx);
            }
            //hover
            let mouse_color = [0.9, 1.0, 0.9, 0.1];//light green
            graphics::rectangle(mouse_color,  to_f64_4(mouse_pos[0], mouse_pos[1], 1, 1),  transform,  gfx);
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

        // This is a (probably premature) optimization to reuse self.drones
        // and avoid allocating and freing every time.
        // The functional approach would be to iterate, map into a vector with
        // lengt 0, 1 or 2, flat_map() and then collect().
        let mut i = 0;
        let mut len = self.drones.len();// Don't increase when I add new
        while i < len {
            let m = self.drones[i];
            match self.board[m[1] as usize][m[0] as usize] {
                Open(Some(path)) => {// move along
                    self.drones[i] = vec2_add(m, path.next.unit_vector());
                },
                Open(None) => {// jitter randomly
                    let min = [(m[0] as i32)as f64, (m[1] as i32)as f64];
                    let max = vec2_add(min, [0.6,0.6]);
                    // returns [0,1), if positions seems to decrease, use Open01
                    let x = m[0] + rand::thread_rng().next_f64() - 0.5;
                    let y = m[1] + rand::thread_rng().next_f64() - 0.5;
                    if x >= min[0]  &&  x <= max[0] {
                        self.drones[i][0] = x;
                    }
                    if y >= min[1]  &&  y <= max[1] {
                        self.drones[i][1] = y;
                    }
                },
                Wall => {// remove
                    let last = self.drones.pop().unwrap();
                    if i != len-1 {
                        self.drones[i] = last;
                    }
                    len -= 1;
                    i = i.wrapping_sub(1);
                },
                Target if len < 200 => self.drones.push(m),// clone
                Target => {/*else it gets slow quickly*/},
            }
            i = i.wrapping_add(1);
        }
    }

    /// Recalculates the numbers when you change the destination.
    fn update_paths(&mut self) {
        //reset all
        for tile in self.board.iter_mut().flat_map(|row| row.iter_mut() ) {
            if let Open(Some(_)) = *tile {
                *tile = Open(None);
            }
        }

        if let Some(target) = self.target {
            fn go(board: &mut Board,  p: [i32; 2],
                  from_dist: i32,  from_dir: Direction)
            -> bool {
                if p[0]>=0  &&  p[0]<BOARD_WIDTH
                && p[1]>=0  &&  p[1]<BOARD_HEIGHT {
                    let tile = &mut board[p[1]as usize][p[0]as usize];
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

            let mut to_check : VecDeque<([i32; 2], i32, Direction)> = VecDeque::new();
            to_check.push_back((target, 0, South));
            while let Some((from_pos, from_dist, from_dir)) = to_check.pop_front() {
                if go(&mut self.board,  from_pos,  from_dist, from_dir) {
                    to_check.push_back((vec2_add(from_pos, North.unit_vector()), from_dist+1, South));
                    to_check.push_back((vec2_add(from_pos, South.unit_vector()), from_dist+1, North));
                    to_check.push_back((vec2_add(from_pos, West.unit_vector()), from_dist+1,  East));
                    to_check.push_back((vec2_add(from_pos, East.unit_vector()), from_dist+1,  West));
                }
            }
        }
    }

    fn mouse_move(&mut self,  pos: Option<[i32; 2]>) {
        self.mouse_pos = pos;
        if pos.is_none() {// left the window
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

                    let from = self.board[start[1] as usize][start[0] as usize];
                    let set = match from {Open(_)=>{Wall} Wall=>{Open(None)} Target=>{return}};

                    let (first, second) = Game::order_points(start, end);
                    for tile in self.board[first[1]as usize .. 1+second[1]as usize].iter_mut()
                                    .flat_map(|row| row[first[0]as usize .. 1+second[0]as usize].iter_mut() )
                                    .filter(|tile| **tile != Target ) {
                        *tile = set;
            }   }   }
            (MouseButton::Right, Some(pos))  =>  {
                let mut set = true;
                if let Some(target) = self.target {
                    self.board[target[1] as usize][target[0] as usize] = Open(None);
                    self.target = None;
                    set = pos != target;
                }
                if set {
                    self.board[pos[1] as usize][pos[0] as usize] = Target;
                    self.target = Some(pos);
                }
            }
            (_,_) => {}
        }
        self.update_paths();
    }

    fn key_press(&mut self,  key: Key) {
        if key == Key::P {
            self.paused = !self.paused;
        }
    }
}

use piston::window::WindowSettings;
use piston::event_loop::Events;
use piston::input::{Button, Motion, Input};
use opengl_graphics::OpenGL;
use graphics::draw_state::Blend;

extern crate piston_window;
use piston_window::PistonWindow;

// Handles setup, resize and converting mouse coordinates to tile coordinates.
fn main() {
    println!("Left click to move or remove tha yellow target.");
    println!("Right click to place or remove walls.");
    println!("(You can also select multiple tiles.)");
    println!("");
    println!("Press p to pause");

    let mut window: PistonWindow =
        WindowSettings::new("PistonPath", [
                INITIAL_TILE_SIZE as u32  *  BOARD_WIDTH as u32,
                INITIAL_TILE_SIZE as u32  *  BOARD_HEIGHT as u32
            ]).exit_on_esc(true).build().unwrap();

    let mut gfx = GlGraphics::new(OpenGL::V3_2);

    let mut tile_size = INITIAL_TILE_SIZE;//changes if window is resized
    let mut offset = [0.0; 2];//letterboxing after resize

    let mut game = Game::new();
    let mut event_loop: Events = window.events;
    while let Some(e) = event_loop.next(&mut window) {
        match e {
            Input::Render(render_args/*: RenderArgs*/) => {
                let context: Context = Context::new_viewport(render_args.viewport())
                                               .trans(offset[0], offset[1])
                                               .scale(tile_size, tile_size);
                //by default alpha blending is disabled, which means all semi-transparent colors are considered opaque.
                //since colors are blended pixel for pixel, this has a performance cost,
                //the alternative is to check for existing color in tile, and blend manually, or even statically
                context.draw_state.blend(Blend::Alpha);

                game.render(context.draw_state, context.transform, &mut gfx);
            }
            Input::Update(update_args) => {
                game.update(update_args.dt);//deltatime is its only field
            }

            Input::Press(Button::Keyboard(key)) => {
                game.key_press(key);
            }
            Input::Press(Button::Mouse(button)) => {
                game.mouse_press(button);
            }
            Input::Release(Button::Mouse(button)) => {
                game.mouse_release(button);
            }
            Input::Resize(x,y) => {/*x and y are u32*/
                tile_size = f64::min(x as f64 / (BOARD_WIDTH as f64),
                                     y as f64 / (BOARD_HEIGHT as f64));
                offset = [(x as f64 - tile_size*BOARD_WIDTH as f64) / 2.0,
                          (y as f64 - tile_size*BOARD_HEIGHT as f64) / 2.0];
                gfx.viewport(0, 0, x as i32, y as i32);
            }
            Input::Move(Motion::MouseCursor(x,y)) => {
                let mut pos = None;
                // compare floats to avoid rounding at the edges
                let x = (x - offset[0]) / tile_size;
                let y = (y - offset[1]) / tile_size;
                if x >= 0.0  &&  x < BOARD_WIDTH as f64
                && y >= 0.0  &&  y < BOARD_HEIGHT as f64 {
                    pos = Some([x as i32, y as i32]);
                }
                game.mouse_move(pos);
            }
            Input::Cursor(_) => {//only happens if a button is pressed
                game.mouse_move(None);
            }

            _ => {}
        }
    }
}
