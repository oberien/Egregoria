#![allow(clippy::too_many_arguments)]
#![allow(clippy::float_cmp)]

use crate::context::Context;
use crate::logger::MyLog;
use log::LevelFilter;

#[macro_use]
extern crate common;

#[macro_use]
extern crate egregoria;

mod audio;
mod context;
mod game_loop;
mod gui;
mod input;
mod logger;
mod rendering;

fn main() {
    let leaked = Box::leak(Box::new(MyLog::new()));
    log::set_logger(leaked).unwrap();
    log::set_max_level(LevelFilter::Debug);
    log_panics::init();

    let mut ctx = Context::new();

    let state = game_loop::State::new(&mut ctx);
    ctx.start(state);
}
