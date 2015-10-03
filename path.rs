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
const UPDATE_TIME: f64 = 0.15;


extern crate vecmath;
use vecmath::Vector2;

type Point = Vector2<i32>;
trait Point2<T> {
    fn x(&self) -> T;
    fn y(&self) -> T;
}
impl Point2<i32> for Point {
    fn x(&self) -> i32 {self[0]}
    fn y(&self) -> i32 {self[1]}
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


extern crate piston;
use piston::input::keyboard::Key;
use piston::input::mouse::MouseButton;

extern crate graphics;
use graphics::{Context,color,math};
use graphics::math::Matrix2d;
use graphics::types::Color;

extern crate opengl_graphics;
use opengl_graphics::GlGraphics;

struct Game {
    board : [[Tile; BOARD_WIDTH]; BOARD_HEIGHT],
    target : Option<Point>,
    paused : bool,
    time: f64,
    update_time: f64,
} impl Game {
    fn new() -> Game {
        Game {time: UPDATE_TIME,
              update_time: UPDATE_TIME,
              paused: false,
              target: None,
              board: [[Tile::Open(None); BOARD_WIDTH]; BOARD_HEIGHT],
        }
    }

    fn render(&mut self,  tile_size: f64,  transform: math::Matrix2d, gfx: &mut GlGraphics) {
        extern crate glutin_window;
        use graphics::{clear, rectangle};

        clear(color::hex("000000"), gfx);

        //tiles
        for (y,ref row) in self.board.into_iter().enumerate() {
            for (x,tile) in row.into_iter().enumerate() {
                graphics::rectangle(
                    tile.color(),
                    graphics::rectangle::square(
                        x as f64 * tile_size,
                        y as f64 * tile_size,
                        tile_size
                    ),
                    transform, gfx
                );
            }
        }
    }

    fn update(&mut self, dt: f64) {
        if self.paused {
            return;
        }
        self.time += dt;
    }

    fn mouse_click(&mut self,  pos : Point,  button: MouseButton) {
        let tile = &mut self.board[pos.y() as usize][pos.x() as usize];
        match (button, *tile) {
            (MouseButton::Left, Open(_)) => {*tile = Wall}
            (MouseButton::Left, Wall) => {*tile = Open(None)}
            (MouseButton::Right, Target) => {
                *tile = Open(None);
                self.target = None;
            }
            (MouseButton::Right, Open(_)) => {
                *tile = Target;
                self.target = Some(pos);
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
    let mut mouse_pos: (f64, f64) = (std::f64::NAN, std::f64::NAN);
    let mut tile_size = TILE_SIZE;

    let mut game = Game::new();
    for e in window.events() {
        match e {
            Event::Render(render_args/*: RenderArgs*/) => {
                let transform: Matrix2d = Context::new_viewport(render_args.viewport()).transform;
                //update if window has been resized, else weird thing would happen
                //THANK YOU Arcterus/game-of-life/src/app.rs
                &mut gfx.viewport(0, 0, render_args.width as i32, render_args.height as i32);

                game.render(tile_size, transform, &mut gfx);
            }
            Event::Update(update_args) => {
                game.update(update_args.dt);//deltatime is its only field
            }
            Event::Input(Input::Press(Button::Keyboard(key))) => {
                game.key_press(key);
            }
            Event::Input(Input::Press(Button::Mouse(button))) => {
                let (x, y) = mouse_pos;
                let tile = [(x/tile_size) as i32, (y/tile_size) as i32];
                if tile.x() >= 0  &&  tile.x() < BOARD_WIDTH as i32
                && tile.y() >= 0  &&  tile.y() < BOARD_HEIGHT as i32 {
                    game.mouse_click(tile, button);
                }// else click in the black area when the window has been resized
            }
            Event::Input(Input::Resize(x,y)) => {
                tile_size = f64::min( x as f64 / (BOARD_WIDTH as f64),  y as f64 / (BOARD_HEIGHT as f64));
            }
            Event::Input(Input::Move(Motion::MouseCursor(x,y))) => {
                mouse_pos = (x,y);
            }
            _ => {}
        }
    }
}
