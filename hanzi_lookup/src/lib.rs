#![allow(dead_code)]
#![allow(unused_imports)]

extern crate wasm_bindgen;
extern crate serde_derive;
extern crate bincode;

mod analyzed_character;
mod cubic_curve_2d;
mod entities;
mod match_collector;
mod matcher;

use serde_derive::{Deserialize, Serialize};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

use match_collector::*;
use analyzed_character::*;
use match_collector::*;
use matcher::*;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::fmt::Write;
use std::ffi::CStr;
use std::ptr;

#[derive(Serialize, Deserialize)]
struct Action {
    action: String,
    points: Vec<Vec<u8>>,
}

#[derive(Serialize, Deserialize)]
struct Input {
    char: String,
    ix: i64,
    duration: i64,
    actions: Vec<Action>,
}


#[wasm_bindgen]
pub fn lookup(input: &JsValue, limit: usize) -> String {
    // Input is vector of vector of vector of numbers - how strokes and their points are represented in JS
    let input: Vec<Vec<Vec<f32>>> = input.into_serde().unwrap();
    // Convert to typed form: vector of strokes
    let mut strokes: Vec<Stroke> = Vec::with_capacity(input.len());
    for i in 0..input.len() {
        let mut stroke = Stroke {
            points: Vec::with_capacity(input[i].len()),
        };
        for j in 0..input[i].len() {
            stroke.points.push(Point {
                x: input[i][j][0].round() as u8,
                y: input[i][j][1].round() as u8,
            });
        }
        strokes.push(stroke);
    }
    let lookup_res = match_typed(&strokes, limit);
    serde_json::to_string(&lookup_res).unwrap()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: u8,
    pub y: u8,
}

#[derive(Debug)]
pub struct Stroke {
    pub points: Vec<Point>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Match {
    pub hanzi: char,
    pub score: f32,
}

thread_local!(static MATCHER: RefCell<Matcher> = RefCell::new(Matcher::new()));

pub fn match_typed(strokes: &Vec<Stroke>, limit: usize) -> Vec<Match> {
    let mut res: Vec<Match> = Vec::with_capacity(limit);
    let mut collector = MatchCollector::new(&mut res, limit);
    MATCHER.with(|matcher| {
        matcher.borrow_mut().lookup(strokes, &mut collector);
    });
    res
}

const ITERS: usize = 10;
const RESULTS: usize = 20;

fn parse_sample(str_strokes: &str) -> Vec<Stroke> {
    let vec_strokes: Vec<Vec<Vec<u8>>> = serde_json::from_str(str_strokes).unwrap();
    let mut strokes: Vec<Stroke>  = Vec::new();
    for vec_stroke in &vec_strokes {
        let mut points: Vec<Point> = Vec::new();
        for vec_point in vec_stroke {
            points.push(Point {
                x: vec_point[0],
                y: vec_point[1],
            });
        }
        strokes.push(Stroke {
            points: points,
        });
    }
    strokes
}

fn clone_stroke(stroke: &Stroke) -> Stroke {
    let mut res = Stroke {
        points: Vec::with_capacity(stroke.points.len()),
    };
    for i in 0..stroke.points.len() {
        res.points.push(Point {
            x: stroke.points[i].x,
            y: stroke.points[i].y,
        });
    }
    res
}

fn incremental_replay(chars: &Vec<Vec<Stroke>>) -> Vec<Vec<Stroke>> {
    let mut res: Vec<Vec<Stroke>> = Vec::new();
    for i in 0..chars.len() {
        let this_char = &chars[i];
        for j in 1..this_char.len() {
            res.push(Vec::new());
            let strokes: &mut Vec<Stroke> = res.last_mut().unwrap();
            for k in 0 ..j {
                strokes.push(clone_stroke(&this_char[k]));
            }
        }
    }
    res
}

#[no_mangle]
pub extern "C" fn c_lib_main(results: libc::size_t, input: *const libc::c_char) -> *const libc::c_char {
    let input_str: &std::ffi::CStr = unsafe { std::ffi::CStr::from_ptr(input) };
    let inputs: Vec<Vec<Stroke>> = vec![parse_sample(&input_str.to_str().unwrap())];

    //let inputs = incremental_replay(&res);

    let matches = match_typed(&inputs.last().unwrap(), results);
    let mut chars = String::new();

    for i in 0..matches.len() {
        write!(chars, "{}", matches[i].hanzi).unwrap();
    }

    let c_str = std::ffi::CString::new(chars).expect("Fail");
    c_str.into_raw()
}

#[no_mangle]
pub extern "C" fn c_lib_main_cleanup(ptr: *const libc::c_char) {
    unsafe { let _ = std::ffi::CString::from_raw(ptr as *mut _); };
}

pub fn c_lib_main_n(results: libc::size_t) {
    let str = std::ffi::CString::new("debug/inputs.txt").expect("string");
    let file = File::open(str.to_str().unwrap()).expect("Failed to open file");

    for line in BufReader::new(file).lines() {
        let line = line.expect("Line");

        let l1 = std::ffi::CString::new(line).unwrap() ;

        let r: *const libc::c_char = c_lib_main(results, l1.as_ptr());
        c_lib_main_cleanup(r);
    }
}
