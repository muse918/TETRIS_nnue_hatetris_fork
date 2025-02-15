#![allow(unused_parens)]

use crate::constants::{
    AEON, ALL_CONV, CHECKPOINTS, EFF_HEIGHT, HIDDEN, MASTER_BEAM_DEPTH, MASTER_BEAM_WIDTH,
    MASTER_MAX_PLAY, MULTIPLIER, TRAINING_BEAM_DEPTH, TRAINING_BEAM_WIDTH, TRAINING_MAX_PLAY,
};

use std::{cmp::Ordering, fmt::Debug};
//use std::{arch::x86_64::__m256d, simd::f64x4};

use rand::thread_rng;
use rand_distr::{Distribution, Normal};
use savefile_derive::Savefile;

pub type RowT = u16;
pub type WaveT = u64;
pub type WellT = [RowT; EFF_HEIGHT];
pub type ScoreT = u16;

#[derive(Clone, Debug, Eq, Hash, PartialEq, Savefile)]
pub struct State {
    pub well: WellT,
    pub score: ScoreT,
}

impl State {
    pub fn new() -> State {
        return State {
            well: [0; EFF_HEIGHT],
            score: 0,
        };
    }

    pub fn convert(state: StateH) -> State {
        return State {
            well: state.well,
            score: state.score,
        };
    }
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        let first_cmp = self.score.cmp(&other.score);
        if first_cmp != Ordering::Equal {
            return first_cmp;
        }

        let mut second_cmp = Ordering::Equal;
        let mut i = 0;
        while second_cmp == Ordering::Equal && i < EFF_HEIGHT {
            second_cmp = self.well[i].cmp(&other.well[i]);
            i += 1;
        }

        return second_cmp;
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Hash)]
pub struct StateH {
    pub well: WellT,
    pub score: ScoreT,
    pub heuristic: i64,
}

impl PartialEq for StateH {
    fn eq(&self, other: &Self) -> bool {
        if self.score != self.score {
            return false;
        } else {
            for i in 0..EFF_HEIGHT {
                if self.well[i] != other.well[i] {
                    return false;
                }
            }
        }
        return true;
    }
}

impl Eq for StateH {}

impl Ord for StateH {
    fn cmp(&self, other: &Self) -> Ordering {
        let first_cmp = self.heuristic.cmp(&other.heuristic);
        if first_cmp != Ordering::Equal {
            return first_cmp;
        }

        let second_cmp = self.score.cmp(&other.score);
        if second_cmp != Ordering::Equal {
            return second_cmp;
        }

        let mut third_cmp = Ordering::Equal;
        let mut i = 0;
        while third_cmp == Ordering::Equal && i < EFF_HEIGHT {
            third_cmp = self.well[i].cmp(&other.well[i]);
            i += 1;
        }

        return third_cmp;
    }
}

impl PartialOrd for StateH {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl StateH {
    pub fn new() -> StateH {
        return StateH {
            well: [0; EFF_HEIGHT],
            score: 0,
            heuristic: i64::MIN,
        };
    }
}

#[derive(Clone, Debug, PartialEq, Savefile)]
pub struct StateP {
    pub well: WellT,
    pub score: ScoreT,
    pub heuristic: f64,
    pub min_prev_heuristic: f64,
    pub depth: usize,
    pub parent_index: usize,
}

impl StateP {
    pub fn convert_state(&self) -> State {
        return State {
            well: self.well.clone(),
            score: self.score,
        };
    }

    pub fn convert_state_h(&self) -> StateH {
        return StateH {
            well: self.well.clone(),
            score: self.score,
            heuristic: (self.heuristic * MULTIPLIER) as i64,
        };
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateD {
    pub well: WellT,
    pub score: ScoreT,
    pub depth: i32,
    pub run_id: i32,
}

impl StateD {
    pub fn convert(state: &State, depth: i32, run_id: i32) -> StateD {
        return StateD {
            well: state.well,
            score: state.score,
            depth: depth,
            run_id: run_id,
        };
    }

    pub fn convert_tuple(state: &StateD) -> (State, (i32, i32)) {
        return (
            State {
                well: state.well,
                score: state.score,
            },
            (state.depth, state.run_id),
        );
    }
}

impl Ord for StateD {
    fn cmp(&self, other: &Self) -> Ordering {
        let first_cmp = self.score.cmp(&other.score);
        if first_cmp != Ordering::Equal {
            return first_cmp;
        }

        let mut second_cmp = Ordering::Equal;
        let mut i = 0;
        while second_cmp == Ordering::Equal && i < EFF_HEIGHT {
            second_cmp = self.well[i].cmp(&other.well[i]);
            i += 1;
        }

        return second_cmp;
    }
}

impl PartialOrd for StateD {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub trait GetInsertQuery {
    fn get_insert_query(&self) -> String;
}

impl GetInsertQuery for StateD {
    fn get_insert_query(&self) -> String {
        let mut query =
            String::from("INSERT INTO WELLS (well_state, run_id, depth, score) VALUES (");

        query.push_str(format!("'{:?}'", self.well).as_str());
        query.push_str(", ");
        query.push_str(&self.run_id.to_string());
        query.push_str(", ");
        query.push_str(&self.depth.to_string());
        query.push_str(", ");
        query.push_str(&self.score.to_string());
        query.push_str(");");
        return query;
    }
}

pub struct StatePP(pub StateH);

impl Debug for StatePP {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for x in self.0.well {
            let mut row = format!("{:010b}\n", x)
                .chars()
                .map(|x| if x == '1' { "#" } else { " " })
                .collect::<String>();
            row.extend("\n".chars());
            f.write_str(&row)?;
        }
        f.write_fmt(format_args!("Heuristic: {}\n", self.0.heuristic))?;
        f.write_fmt(format_args!("Score: {}\n", self.0.score))?;

        Ok(())
    }
}

#[derive(Clone, Debug, Savefile)]
pub struct WeightT {
    pub conv: Vec<[f64; HIDDEN]>,
    pub hidden: [f64; HIDDEN],
}

#[derive(Clone, Debug, Savefile)]
pub struct WeightDiscreteT {
    pub conv: Vec<[i16; HIDDEN]>,
    pub hidden: [i16; HIDDEN],
}

// #[derive(Clone, Debug)]
// pub struct WeightChunkT {
// 	pub conv: Vec<Vec<__m256d>>,
// 	pub hidden: Vec<__m256d>
// }

impl WeightDiscreteT {
    pub fn zero() -> WeightDiscreteT {
        return WeightDiscreteT {
            conv: vec![[0; HIDDEN]; ALL_CONV],
            hidden: [0; HIDDEN],
        };
    }
}

impl WeightT {
    pub fn zero() -> WeightT {
        return WeightT {
            conv: vec![[0.0; HIDDEN]; ALL_CONV],
            hidden: [0.0; HIDDEN],
        };
    }

    pub fn new() -> WeightT {
        let mut new_weights = WeightT::zero();
        let mut rng = thread_rng();

        let dist_conv = Normal::new(0.0, 1.0 / (ALL_CONV as f64).sqrt()).unwrap();
        let dist_hidden = Normal::new(0.0, 1.0 / (HIDDEN as f64).sqrt()).unwrap();

        for c in 0..ALL_CONV {
            for h in 0..HIDDEN {
                new_weights.conv[c][h] = dist_conv.sample(&mut rng);
            }
        }
        for h in 0..HIDDEN {
            new_weights.hidden[h] = dist_hidden.sample(&mut rng);
        }
        return new_weights;
    }

    pub fn to_discrete_network(&self) -> WeightDiscreteT {
        let mut new_weights = WeightDiscreteT::zero();
        let conversion_factor = 63.0 / 64.0;

        for c in 0..ALL_CONV {
            for h in 0..HIDDEN {
                let tmp = self.conv[c][h] * 2048.0 * conversion_factor;
                new_weights.conv[c][h] = match tmp {
                    x if x >= (i8::MAX as f64) => i8::MAX,
                    x if x <= (-i8::MAX as f64) => -i8::MAX,
                    _ => (tmp as i8),
                } as i16;
            }
        }

        for h in 0..HIDDEN {
            let tmp2 = self.hidden[h] * 256.0;
            new_weights.hidden[h] = match tmp2 {
                x if x >= (i8::MAX as f64) => i8::MAX,
                x if x <= (-i8::MAX as f64) => -i8::MAX,
                _ => (tmp2 as i8),
            } as i16;
        }
        return new_weights;
    }

    // pub fn to_chunks(&self) -> WeightChunkT {
    // 	let mut conv = vec![];
    // 	for i in 0..self.conv.len() {
    // 		conv.push(vec![]);
    // 		for j in (0..self.conv[i].len()).step_by(CHUNK) {
    // 			conv[i].push(__m256d::from(f64x4::from_slice(&self.conv[i][j..j+CHUNK])));
    // 		}
    // 	}

    // 	let mut hidden = vec![];
    // 	for j in (0..self.hidden.len()).step_by(CHUNK) {
    // 		hidden.push(__m256d::from(f64x4::from_slice(&self.hidden[j..j+CHUNK])));
    // 	}

    // 	return WeightChunkT { conv: conv, hidden: hidden }
    // }
}

#[derive(Clone, Debug)]
pub struct SearchConf {
    pub beam_width: usize,
    pub beam_depth: usize,
    pub generation: usize,
    pub max_play: usize,
    pub quiescent: bool,
    pub parent: bool,
    pub save: bool,
    pub print: bool,
}

impl SearchConf {
    pub fn master(generation: usize) -> SearchConf {
        return SearchConf {
            beam_width: MASTER_BEAM_WIDTH,
            beam_depth: MASTER_BEAM_DEPTH,
            generation: generation,
            max_play: MASTER_MAX_PLAY,
            quiescent: true,
            parent: true,
            save: true,
            print: true,
        };
    }

    pub fn training(generation: usize) -> SearchConf {
        return SearchConf {
            beam_width: TRAINING_BEAM_WIDTH,
            beam_depth: TRAINING_BEAM_DEPTH,
            generation: generation,
            max_play: TRAINING_MAX_PLAY,
            quiescent: false,
            parent: false,
            save: false,
            print: false,
        };
    }

    pub fn testing() -> SearchConf {
        return SearchConf {
            beam_width: MASTER_BEAM_WIDTH,
            beam_depth: usize::MAX,
            generation: 0,
            max_play: MASTER_MAX_PLAY,
            quiescent: true,
            parent: true,
            save: false,
            print: true,
        };
    }

    pub fn run_name(&self) -> String {
        return format!("aeon-{}-gen-{}", AEON, self.generation);
    }

    pub fn aeon_path(&self) -> String {
        return format!("{}/Aeon {}", CHECKPOINTS, AEON);
    }

    pub fn generation_path(&self) -> String {
        return format!("{}/Generation {}", self.aeon_path(), self.generation);
    }

    pub fn neural_network_path(&self) -> String {
        return format!("{}/Network {}.bin", self.generation_path(), self.generation);
    }

    pub fn replay_path(&self) -> String {
        return format!("{}/Replay", self.generation_path());
    }

    pub fn training_path(&self) -> String {
        return format!("{}/Training", self.generation_path());
    }

    pub fn move_path(&self, depth: usize) -> String {
        return format!("{}/move_{}.bin", self.replay_path(), depth);
    }

    pub fn parent_path(&self, depth: usize) -> String {
        return format!("{}/parent_{}.bin", self.replay_path(), depth);
    }

    pub fn epoch_path(&self, epoch: isize) -> String {
        return format!("{}/epoch_{}.bin", self.training_path(), epoch);
    }

    pub fn data_path(&self) -> String {
        return format!("{}/all_epochs.bin", self.training_path());
    }
}
