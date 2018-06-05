/* Copyright (C) 2015 Alexandru Cojocaru,
 *               2018 Torbjørn Birch Moltu
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

// font-loader's API is too limitied to express "any sans-serif font, ideally monospace"
const FONT_NAME: &'static str = "arial";
const DIGIT_ASPECT_RATIO: f64 = 0.71; // observed width/height
const MAX_DIGITS_SCALE: usize = 2; // don't increase digit size further when distance < 10
const FONT_RESOLUTION: f64 = 100.0; // glyph height in pixels
const BORDER_RADIUS: f64 = 0.03; // relative to tile_size
const TILE_MIN_PADDING: f64 = 0.08;
const INITIAL_TILE_SIZE: f64 = 50.0;

const BOARD_WIDTH: i32 = 20;
const BOARD_HEIGHT: i32 = 15;
const UPDATE_TIME: f64 = 0.20;


use std::ops::Neg;
use std::cmp;
use std::time::Instant;
use std::collections::vec_deque::VecDeque;
extern crate num;
use num::{Zero,One,ToPrimitive};
extern crate vecmath;
use vecmath::vec2_add; // Vector2 is [T; 2]
// Why not just use `+`?
// Any math library in rust have to make a choice: Either use primitive slices or tuples
// which make constructing and destructuring pain-free, or use std::ops::*.
// Rusts coherence rules prevents them for doing both:
// You cannot implement an external trait for a foreign type.
extern crate piston_window;
use piston_window::{Context,DrawState,Transformed,color,math}; // from piston2d-graphics
use piston_window::types::Color; // from piston2d-graphics
use piston_window::{MouseButton,Key};// from piston::input
use piston_window::{Event,Loop,RenderArgs,UpdateArgs,Input}; // from piston_input
use piston_window::{ButtonArgs,ButtonState,Button,Motion}; // from piston_input
use piston_window::draw_state::Blend; // from piston2d-graphics
use piston_window::WindowSettings; // from piston::window
use piston_window::Events; // from piston::event_loop
use piston_window::PistonWindow; // from piston_window
use piston_window::TextureSettings; // from graphicsz65lw
extern crate opengl_graphics;
use opengl_graphics::{GlGraphics,GlyphCache,OpenGL};
extern crate rand;
use rand::{Rng,FromEntropy};
use rand::rngs::SmallRng;
use rand::distributions::Open01;
extern crate font_loader;
use font_loader::system_fonts::{FontProperty,FontPropertyBuilder};


#[derive(Clone,Copy, PartialEq,Eq)]
enum Direction {North, South, East, West}
use self::Direction::*;
impl Direction {
    /// Is generic so it can produce both floats and integers
    fn unit_vector<T:Zero+One+Neg<Output=T>>(self) -> [T; 2] {
        match self {
            North => [T::zero(),       T::one()      ],
            South => [T::zero(),       T::one().neg()],
            East  => [T::one(),        T::zero()     ],
            West  => [T::one().neg(),  T::zero()     ],
        }
    }
}


#[derive(Clone,Copy, PartialEq,Eq)]
struct Path {
    distance: i32,
    next: Direction,
}

#[derive(Clone,Copy, PartialEq,Eq)]
enum Tile {
    Wall,
    Target,
    Open(Option<Path>),
}
use self::Tile::*; // use Wall instead of Tile::Wall
impl Tile {
    fn color(&self) -> Color { match *self {
        Wall    => color::hex("002951"),
        Target  => color::hex("8ba673"),
        Open(_) => color::hex("001122"),
    }}
}


type Board = [[Tile; BOARD_WIDTH as usize]; BOARD_HEIGHT as usize];
// Contains the game logic
struct Game<'a> {
    board: Board,
    drones: Vec<[f64; 2]>,
    target: Option<[i32; 2]>,
    mouse_pos: Option<[i32; 2]>,
    selection_start: Option<[i32; 2]>,
    paused: bool,
    time: f64,
    update_time: f64,
    rng: SmallRng,
    character_cache: GlyphCache<'a>,
} impl<'a> Game<'a> {
    fn new(font_data: &[u8]) -> Game {
        let mut g = Game {
            character_cache: GlyphCache::from_bytes(font_data, (), TextureSettings::new()).unwrap(),
            rng: SmallRng::from_entropy(),
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
        return g; // Just `g` would do, but to me looks unfinished below `g.something`.
    }             // like I forgot to write the rest of the function.

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

        piston_window::clear(color::BLACK, gfx); // comment out and see!

        // tiles
        for (y_usize,ref row) in self.board.into_iter().enumerate() {
            for (x_usize,tile) in row.into_iter().enumerate() {
                let (x,y) = (x_usize as f64, y_usize as f64);
                piston_window::rectangle(tile.color(), [x,y,1.0,1.0], transform, gfx);
                if let Open(Some(path)) = *tile {
                    // number rendering
                    let as_str: &str = &path.distance.to_string()[..];
                    let digits = as_str.len(); // digits aren't unicode
                    let show_digits = usize::max(digits, MAX_DIGITS_SCALE);
                    const AVAILABLE_DIGIT_HEIGHT: f64 = 1.0-2.0*(TILE_MIN_PADDING+BORDER_RADIUS);
                    const AVAILABLE_BOX_WIDTH: f64 = AVAILABLE_DIGIT_HEIGHT;
                    let available_digit_width = AVAILABLE_BOX_WIDTH / show_digits as f64;
                    let digit_height = f64::min(AVAILABLE_DIGIT_HEIGHT, 
                                                available_digit_width / DIGIT_ASPECT_RATIO);
                    let scale_factor = digit_height / FONT_RESOLUTION;
                    let digit_width = digit_height * DIGIT_ASPECT_RATIO;
                    let box_width = digit_width*digits as f64;
                    let bottom_padding = (1.0-digit_height) / 2.0;
                    let left_box_padding = (1.0-box_width) / 2.0;
                    let left_padding = left_box_padding;
                    // debugging rectangles
                    // piston_window::rectangle(color::hex("226688"), [x+(1.0-AVAILABLE_BOX_WIDTH)/2.0,y+(1.0-AVAILABLE_DIGIT_HEIGHT)/2.0,AVAILABLE_BOX_WIDTH,AVAILABLE_DIGIT_HEIGHT], transform, gfx);
                    // piston_window::rectangle(color::hex("2266aa"), [x+(1.0-AVAILABLE_BOX_WIDTH)/2.0,y+(1.0-AVAILABLE_DIGIT_HEIGHT)/2.0,available_digit_width,AVAILABLE_DIGIT_HEIGHT], transform, gfx);
                    // piston_window::rectangle(color::hex("668888"), [x+left_box_padding,y+bottom_padding,box_width,digit_height], transform, gfx);
                    // piston_window::rectangle(color::hex("6688aa"), [x+left_box_padding,y+bottom_padding,digit_width,digit_height], transform, gfx);
                    let char_pos = transform
                        .trans(x + left_padding,  1.0 + y - bottom_padding)
                        .scale(scale_factor, scale_factor);
                    piston_window::text::Text::new_color(Target.color(), FONT_RESOLUTION as u32)
                        .draw(as_str, &mut self.character_cache, &draw_state, char_pos, gfx)
                        .unwrap();
                }
            }
        }

        // drones
        for p in &self.drones {
            let red = color::hex("ee2222");
            let brown = color::hex("330000");
            let border = [p[0],p[1],0.4,0.4];
            let main = [p[0]+0.05,p[1]+0.05,0.3,0.3];
            piston_window::rectangle(brown, border, transform, gfx);
            piston_window::rectangle(red, main, transform, gfx);
        }

        // hover highlight and selection
        if let Some(mouse_pos) = self.mouse_pos {
            // selection
            if let Some(start) = self.selection_start {
                let (a,b) = Game::order_points(start, mouse_pos);
                let rect = to_f64_4(a[0], a[1],  b[0]-a[0]+1, b[1]-a[1]+1);
                let selection_color = [1.0, 1.0, 1.0, 0.2]; // white
                piston_window::rectangle(selection_color, rect, transform, gfx);
            }
            // hover
            let mouse_color = [0.9, 1.0, 0.9, 0.1]; // light green
            piston_window::rectangle(mouse_color,  to_f64_4(mouse_pos[0], mouse_pos[1], 1, 1),  transform,  gfx);
        }

        // border lines
        let line_color = [0.4, 0.4, 0.4, 0.3]; // grey
        for y in 1..BOARD_HEIGHT {
            piston_window::line(line_color, BORDER_RADIUS, to_f64_4(0,y,BOARD_WIDTH,y),  transform, gfx);
        }
        for x in 1..BOARD_WIDTH {
            piston_window::line(line_color, BORDER_RADIUS, to_f64_4(x,0,x,BOARD_HEIGHT),  transform, gfx);
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
                    let x = m[0] + self.rng.sample::<f64,_>(Open01) - 0.5;
                    let y = m[1] + self.rng.sample::<f64,_>(Open01) - 0.5;
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

    /// Recalculates the numbers when the destination you change the destination.
    fn update_paths(&mut self) {
        // reset all
        for tile in self.board.iter_mut().flat_map(|row| row.iter_mut() ) {
            if let Open(Some(_)) = *tile {
                *tile = Open(None);
            }
        }

        if let Some(target) = self.target {
            fn go(board: &mut Board,  p: [i32; 2],  from_dist: i32,  from_dir: Direction) -> bool {
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
                        true // initial tile
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


// Handles setup, resize and converting mouse coordinates to tile coordinates.
fn main() {
    println!("Left click to place or remove walls,");
    println!(" drag to select multiple tiles.");
    println!("Right click to move or remove tha yellow target.");
    println!("Press p to pause");

    let window_size = [
        INITIAL_TILE_SIZE as u32  *  BOARD_WIDTH as u32,
        INITIAL_TILE_SIZE as u32  *  BOARD_HEIGHT as u32
    ];
    let mut window: PistonWindow = // <GlutinWindow>
        WindowSettings::new("PistonPath", window_size)
        .exit_on_esc(true)
        .vsync(true)
        .build()
        .unwrap();

    let mut gfx = GlGraphics::new(OpenGL::V3_2);

    let mut tile_size = INITIAL_TILE_SIZE; // changes if window is resized
    let mut offset = [0.0; 2]; // letterboxing after resize

    let mut font_requirements: FontProperty = FontPropertyBuilder::new().family(FONT_NAME).build();
    let font_data: Vec<u8> = font_loader::system_fonts::get(&mut font_requirements).unwrap().0;

    let mut game = Game::new(&*font_data);
    let mut frames = 0;
    let started = Instant::now();
    let mut event_loop: Events = window.events;
    while let Some(e) = event_loop.next(&mut window) {
        match e {
            Event::Loop(Loop::Render(render_args)) => {
                let render_args: RenderArgs = render_args;
                frames += 1;

                // An optimization introduced in opengl_graphics 0.39.1 causes
                // severe glitching if not wrapped in .draw.
                // (calling it afterwards with an empty closure seems to work too)
                gfx.draw(render_args.viewport(), |context, gfx| {
                    let context: Context = context;
                    let gfx: &mut GlGraphics = gfx; // the same instance as outside
                    // Handle resized windows by scaling and letterboxing.
                    let context: Context = context.trans(offset[0], offset[1])
                                                  .scale(tile_size, tile_size);

                    // By default alpha blending is disabled, which means all
                    // semi-transparent colors are considered opaque.
                    // Since colors are blended pixel for pixel, this has a
                    // performance cost. Alternatively we could check for
                    // existing color in tile, and blend with that.
                    context.draw_state.blend(Blend::Alpha);
                    game.render(context.draw_state, context.transform, gfx);
                });
            }
            Event::Loop(Loop::Update(UpdateArgs{dt})) => {
                game.update(dt);
            }

            Event::Input(Input::Button(ButtonArgs{state,button,..})) => {
                match (button, state) {
                    (Button::Keyboard(key), ButtonState::Press) => game.key_press(key),
                    (Button::Mouse(button), ButtonState::Press) => game.mouse_press(button),
                    (Button::Mouse(button), ButtonState::Release) => game.mouse_release(button),
                    _ => {}
                }
            }
            Event::Input(Input::Resize(x,y)) => {
                let (x,y): (u32,u32) = (x,y);
                tile_size = f64::min(x as f64 / (BOARD_WIDTH as f64),
                                     y as f64 / (BOARD_HEIGHT as f64));
                offset = [(x as f64 - tile_size*BOARD_WIDTH as f64) / 2.0,
                          (y as f64 - tile_size*BOARD_HEIGHT as f64) / 2.0];
                gfx.viewport(0, 0, x as i32, y as i32);
            }
            Event::Input(Input::Move(Motion::MouseCursor(x,y))) => {
                let (x,y): (f64,f64) = (x,y);
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
            Event::Input(Input::Cursor(false)) => {
                // cursor left window, only triggered if a button is pressed.
                game.mouse_move(None);
            }

            _ => {}
        }
    }
    println!();
    println!("average fps: {}", frames / started.elapsed().as_secs());
}
