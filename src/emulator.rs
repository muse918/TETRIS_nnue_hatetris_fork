use crate::constants::{
    EFF_HEIGHT, MAX_ROW, ROTATE_LEFT, ROTATE_RIGHT, WAVE_SIZE, WELL_HEIGHT, WELL_LINE,
};
use crate::masks::{EMPTY_MASKS, HEIGHT_MASKS, ROW_MASKS, SCORE_MASKS};
use crate::neural::{decompose_well, forward_pass};
use crate::pieces::{PIECE_COUNT, PIECE_LIST};
use crate::types::{RowT, ScoreT, SearchConf, State, StateP, WaveT, WeightT, WellT};

use std::cmp::{max, min};

use fnv::{FnvHashMap, FnvHashSet};
use rand::Rng;

// The 'height' of a waveform is the height of the row *below* the bottommost row of the waveform.

pub fn well_slice(height: usize, well: &WellT) -> [RowT; 4] {
    let well_slice = [
        if (height <= 3) {
            0
        } else if (height - 4 >= EFF_HEIGHT) {
            MAX_ROW
        } else {
            well[height - 4]
        },
        if (height <= 2) {
            0
        } else if (height - 3 >= EFF_HEIGHT) {
            MAX_ROW
        } else {
            well[height - 3]
        },
        if (height <= 1) {
            0
        } else if (height - 2 >= EFF_HEIGHT) {
            MAX_ROW
        } else {
            well[height - 2]
        },
        if (height <= 0) {
            0
        } else if (height - 1 >= EFF_HEIGHT) {
            MAX_ROW
        } else {
            well[height - 1]
        },
    ];
    return well_slice;
}

pub fn waveform_to_wells(wave: WaveT, height: usize, p: usize, state: &State) -> Vec<State> {
    let well = state.well;
    let old_score = state.score;

    let mut wells = vec![];
    let mut w = wave;
    for i in (0..WAVE_SIZE).rev() {
        if (w % 2 == 1) {
            let slice = PIECE_LIST[p][i];
            let mut new_well = [0; EFF_HEIGHT];

            let mut score = 0;
            for row in (0..EFF_HEIGHT).rev() {
                let mut new_val = well[row];
                if row <= height - 1 && row + 4 > height - 1 {
                    new_val |= slice[3 - (height - 1 - row)];
                }
                if new_val == MAX_ROW {
                    score += 1;
                } else {
                    new_well[row + score] = new_val;
                }
            }

            wells.push(State {
                well: new_well,
                score: old_score + (score * score) as ScoreT,
            });
        };
        w >>= 1;
    }

    return wells;
}

pub fn waveform_step(w_old: WaveT, p: usize, height: usize, well: &WellT) -> WaveT {
    let well_slice = well_slice(height, well);

    let mut mask = EMPTY_MASKS[p];
    for (r, row) in well_slice.iter().enumerate() {
        mask &= ROW_MASKS[p][*row as usize][r];
    }

    let mut w = w_old & mask;
    let mut w_new = w;
    let mut w_seen = w;
    while w_new > 0 {
        let w_right = w << 4;
        let w_left = w >> 4;
        let w_rotate = ((w & ROTATE_LEFT) << 3) | ((w & ROTATE_RIGHT) >> 1);
        w |= w_right;
        w |= w_left;
        w |= w_rotate;
        w &= mask;
        w_new = w & !w_seen;
        w_seen |= w;

        // TODO: See if there's a way of doing this with only 1 intermediate variable instead of 2.
    }

    return w;
}

pub fn get_well_height(well: &WellT) -> usize {
    let mut height = 0;
    while height < EFF_HEIGHT {
        if well[height as usize] != 0 {
            break;
        };
        height += 1;
    }
    return height;
}

pub fn resting_waveforms(p: usize, well: &WellT) -> Vec<(WaveT, usize)> {
    let mut height = get_well_height(well);

    let mut waves = Vec::with_capacity(EFF_HEIGHT - height + 4);
    let mut w = EMPTY_MASKS[p];

    while w > 0 && height + 1 < WELL_HEIGHT {
        w = waveform_step(w, p, height, &well);
        let h_mask = match height {
            0 => HEIGHT_MASKS[p][3],
            1 => HEIGHT_MASKS[p][2],
            2 => HEIGHT_MASKS[p][1],
            3 => HEIGHT_MASKS[p][0],
            _ => 0,
        };

        waves.push((w & !h_mask, height));
        height += 1;
    }

    // TODO: Incorporate this into the wave list generation, to minimize .push() operations and 0-value waveforms.
    for i in 0..(waves.len() - 1) {
        waves[i].0 &= !waves[i + 1].0;
    }

    return waves;
}

pub fn score_slice(wave: WaveT, height: usize, p: usize, well: &WellT) -> [WaveT; 4] {
    let well_slice = well_slice(height, well);
    let mut score_slice = [0; 4];
    for i in 0..4 {
        score_slice[i] = SCORE_MASKS[p][well_slice[i] as usize][i] & wave;
    }

    return score_slice;
}

pub fn scores(wave: WaveT, height: usize, p: usize, well: &WellT) -> [WaveT; 5] {
    let score_slice = score_slice(wave, height, p, well);

    let mut score = [0; 5];

    score[0] = (!score_slice[0] & !score_slice[1] & !score_slice[2] & !score_slice[3]) & wave;
    if score[0] == wave {
        return score;
    }

    score[1] = (score_slice[0] ^ score_slice[1] ^ score_slice[2] ^ score_slice[3]) & wave;
    if score[1] | score[0] == wave {
        return score;
    }

    score[2] = ((score_slice[0] & score_slice[1])
        ^ (score_slice[0] & score_slice[2])
        ^ (score_slice[0] & score_slice[3])
        ^ (score_slice[1] & score_slice[2])
        ^ (score_slice[1] & score_slice[3])
        ^ (score_slice[2] & score_slice[3]))
        & wave;
    if score[2] | score[1] | score[0] == wave {
        return score;
    }

    score[3] = ((score_slice[0] & score_slice[1] & score_slice[2])
        ^ (score_slice[0] & score_slice[1] & score_slice[3])
        ^ (score_slice[0] & score_slice[2] & score_slice[3])
        ^ (score_slice[1] & score_slice[2] & score_slice[3]))
        & wave;
    if score[3] | score[2] | score[1] | score[0] == wave {
        return score;
    }

    score[4] = (score_slice[0] & score_slice[1] & score_slice[2] & score_slice[3]) & wave;

    return score;
}

pub fn get_wave_height(wave: WaveT, wave_height: usize, p: usize, well: &WellT) -> isize {
    // We only care about the lowest possible height of all the pieces.

    let well_height = get_well_height(well) as isize;
    if wave == 0 {
        return -1 * (WELL_LINE as isize);
    }

    let scores = scores(wave, wave_height, p, well);

    let mut max_height = -1 * (WELL_LINE as isize);

    let mut wsc = [0; 5];
    let mut total = 0;
    for s in 0..5 {
        wsc[s] = scores[s] & wave;

        for (row, h) in HEIGHT_MASKS[p].iter().enumerate() {
            if h & wsc[s] != 0 {
                let tmp_height = min(well_height, (wave_height + row) as isize - 4) + s as isize;
                max_height = max(max_height, tmp_height);
            }
            if h & wsc[s] == wsc[s] {
                break;
            }
        }

        total |= wsc[s];
        if total == wave {
            break;
        };
    }

    return max_height;
}

pub fn get_legal(state: &State) -> (usize, Vec<Vec<(WaveT, usize)>>) {
    let all_waves: Vec<Vec<(WaveT, usize)>> = (0..PIECE_COUNT)
        .map(|p| resting_waveforms(p, &state.well))
        .collect();

    let mut legal_p = 0;
    let mut lowest_height = WELL_HEIGHT as isize;

    for p in 0..PIECE_COUNT {
        let mut piece_height = -1 * (WELL_LINE as isize);
        for wave in &all_waves[p] {
            let new_height = get_wave_height(wave.0, wave.1, p, &state.well);
            if new_height > piece_height {
                piece_height = new_height;
            }
        }
        if piece_height < lowest_height {
            legal_p = p;
            lowest_height = piece_height;
        }
    }

    // random piece; close enough
    (rand::thread_rng().gen_range(0..PIECE_COUNT), all_waves)
    // (legal_p, all_waves)
}

pub fn single_move(state: &State) -> Vec<State> {
    let (piece, all_waves) = get_legal(&state);

    let mut to_return = vec![];
    for (w, h) in &all_waves[piece] {
        let mut w_list = waveform_to_wells(*w, *h, piece, state);
        to_return.append(&mut w_list);
    }

    return to_return;
}

// Gets heuristic for individual well.
// Only to be used when batching is not appropriate.

pub fn network_heuristic_individual(state: &State, weight: &WeightT, conf: &SearchConf) -> f64 {
    let conv_list = decompose_well(&state.well);
    let mut heuristic = forward_pass(conv_list, weight);
    let quiescent = conf.quiescent;

    if !quiescent {
        return heuristic;
    }

    let mut wells_to_evaluate = FnvHashSet::default();
    wells_to_evaluate.insert((state.clone(), 0));

    // TODO: Incorporate final well state cloning into main branch.

    // set max plays
    let mut play_len = 0;

    while wells_to_evaluate.len() > 0 && play_len < conf.max_play {
        let mut queued_wells = FnvHashSet::default();
        for wev in wells_to_evaluate.iter() {
            'piece: for p in 0..PIECE_COUNT {
                let mut tmp_queue = vec![];

                let waves = resting_waveforms(p, &wev.0.well);
                for wave in waves {
                    let slice = score_slice(wave.0, wave.1, p, &wev.0.well);
                    let mut new_w = (0, wave.1);
                    for s in slice {
                        if s > 0 {
                            new_w.0 |= wave.0 & s;
                        }
                    }
                    if new_w.0 > 0 {
                        let new_wells = waveform_to_wells(new_w.0, new_w.1, p, &wev.0);
                        for w in new_wells {
                            if w.score - state.score != wev.1 + 1 {
                                continue 'piece; // If the piece can be used to clear more than 1 line, skip the entire piece.
                            } else {
                                tmp_queue.push(w);
                            }
                        }
                    }
                }

                for well in tmp_queue {
                    if !queued_wells.contains(&(well.clone(), wev.1 + 1)) {
                        queued_wells.insert((well.clone(), wev.1 + 1));

                        let mut tmp_conf = conf.clone();
                        tmp_conf.quiescent = false;

                        let h = network_heuristic_individual(&well, weight, &tmp_conf);
                        heuristic = h.max(heuristic);
                    }
                }

                break 'piece;
            }
        }
        wells_to_evaluate.clear();
        wells_to_evaluate = queued_wells;

        play_len += 1;
    }
    return heuristic;
}

// Used for batches; gets the children and their heuristics.
// This is where loop prevention logic will be.

pub fn network_heuristic(state: &State, weight: &WeightT, conf: &SearchConf) -> Vec<(State, f64)> {
    let legal = single_move(state); // This will be replaced with full piece priority lookback later.
    let quiescent = conf.quiescent;

    let mut heuristics: Vec<(State, f64)> = legal.iter().map(|s| (s.clone(), -1.0)).collect();
    for i in 0..legal.len() {
        let conv_list = decompose_well(&heuristics[i].0.well);
        heuristics[i].1 = forward_pass(conv_list, weight);
    }

    if !quiescent {
        return heuristics;
    }

    let mut wells_to_evaluate = FnvHashMap::default();
    for i in 0..legal.len() {
        if !wells_to_evaluate.contains_key(&(legal[i].clone(), 0)) {
            wells_to_evaluate.insert((legal[i].clone(), 0), vec![i]);
        } else {
            let mut affected_wells = wells_to_evaluate
                .get(&(legal[i].clone(), 0))
                .unwrap()
                .clone();
            affected_wells.push(i);
            wells_to_evaluate.insert((legal[i].clone(), 0), affected_wells);
        }
    }

    let mut heuristic_map = FnvHashMap::default();
    for i in 0..legal.len() {
        heuristic_map.insert(heuristics[i].0.clone(), heuristics[i].1.clone());
    }

    // max play length
    let mut play_len = 0;

    while wells_to_evaluate.len() > 0 && play_len < conf.max_play {
        let mut queued_wells = FnvHashMap::default();
        for wev in wells_to_evaluate.iter() {
            let prev_score = legal[wev.1[0]].score;

            'piece: for p in 0..PIECE_COUNT {
                let mut tmp_queue = vec![];

                let waves = resting_waveforms(p, &wev.0 .0.well);
                for wave in waves {
                    let slice = score_slice(wave.0, wave.1, p, &wev.0 .0.well);
                    let mut new_w = (0, wave.1);
                    for s in slice {
                        if s > 0 {
                            new_w.0 |= wave.0 & s;
                        }
                    }
                    if new_w.0 > 0 {
                        let new_wells = waveform_to_wells(new_w.0, new_w.1, p, &wev.0 .0);
                        for w in new_wells {
                            if w.score - prev_score != wev.0 .1 + 1 {
                                continue 'piece; // If the piece can be used to clear more than 1 line, skip the entire piece.
                            } else {
                                tmp_queue.push(w);
                            }
                        }
                    }
                }

                for well in tmp_queue {
                    if !queued_wells.contains_key(&(well.clone(), wev.0 .1 + 1)) {
                        queued_wells.insert((well.clone(), wev.0 .1 + 1), wev.1.clone());

                        let mut tmp_conf = conf.clone();
                        tmp_conf.quiescent = false;

                        let h = network_heuristic_individual(&well, weight, &tmp_conf);
                        for &id in wev.1 {
                            heuristics[id].1 = heuristics[id].1.max(h);
                        }
                        heuristic_map.insert(well, h);
                    } else {
                        let mut affected_wells = queued_wells
                            .get(&(well.clone(), wev.0 .1 + 1))
                            .unwrap()
                            .clone();
                        let mut to_update = wev.1.clone();
                        affected_wells.append(&mut to_update);
                        queued_wells.insert((well.clone(), wev.0 .1 + 1), affected_wells);
                        let h = *heuristic_map.get(&well).unwrap();
                        for &id in wev.1 {
                            heuristics[id].1 = heuristics[id].1.max(h);
                        }
                    }
                }

                break 'piece;
            }
        }
        wells_to_evaluate.clear();
        wells_to_evaluate = queued_wells;

        play_len += 1;
    }

    return heuristics;
}

pub fn network_heuristic_loop(
    state: &State,
    p: usize,
    parents: &Vec<(usize, StateP)>,
    weight: &WeightT,
    conf: &SearchConf,
) -> (Vec<(State, f64)>, Vec<Vec<State>>) {
    let all_waves: Vec<Vec<(WaveT, usize)>> = (0..PIECE_COUNT)
        .map(|p| resting_waveforms(p, &state.well))
        .collect();

    let mut piece_order = vec![];

    for p in 0..PIECE_COUNT {
        let mut piece_height = -1 * (WELL_LINE as isize);
        for wave in &all_waves[p] {
            let new_height = get_wave_height(wave.0, wave.1, p, &state.well);
            if new_height > piece_height {
                piece_height = new_height;
            }
        }
        piece_order.push((piece_height, p));
    }
    piece_order.sort();

    for (_, legal_p) in piece_order {
        let mut legal = vec![];
        for (w, h) in &all_waves[legal_p] {
            let mut w_list = waveform_to_wells(*w, *h, legal_p, state);
            legal.append(&mut w_list);
        }

        let mut heuristics: Vec<(State, f64)> = legal.iter().map(|s| (s.clone(), -1.0)).collect();
        for i in 0..legal.len() {
            let conv_list = decompose_well(&heuristics[i].0.well);
            heuristics[i].1 = forward_pass(conv_list, weight);
        }

        let mut wells_to_evaluate = FnvHashMap::default();
        for i in 0..legal.len() {
            if !wells_to_evaluate.contains_key(&(legal[i].clone(), 0)) {
                wells_to_evaluate.insert((legal[i].clone(), 0), vec![i]);
            } else {
                let mut affected_wells = wells_to_evaluate
                    .get(&(legal[i].clone(), 0))
                    .unwrap()
                    .clone();
                affected_wells.push(i);
                wells_to_evaluate.insert((legal[i].clone(), 0), affected_wells);
            }
        }

        let mut heuristic_map = FnvHashMap::default();
        for i in 0..legal.len() {
            heuristic_map.insert(heuristics[i].0.clone(), heuristics[i].1.clone());
        }

        let mut play_len = 0;

        while wells_to_evaluate.len() > 0 && play_len < conf.max_play {
            let mut queued_wells = FnvHashMap::default();
            for wev in wells_to_evaluate.iter() {
                let prev_score = legal[wev.1[0]].score;

                'piece: for p in 0..PIECE_COUNT {
                    let mut tmp_queue = vec![];

                    let waves = resting_waveforms(p, &wev.0 .0.well);
                    for wave in waves {
                        let slice = score_slice(wave.0, wave.1, p, &wev.0 .0.well);
                        let mut new_w = (0, wave.1);
                        for s in slice {
                            if s > 0 {
                                new_w.0 |= wave.0 & s;
                            }
                        }
                        if new_w.0 > 0 {
                            let new_wells = waveform_to_wells(new_w.0, new_w.1, p, &wev.0 .0);
                            for w in new_wells {
                                if w.score - prev_score != wev.0 .1 + 1 {
                                    continue 'piece; // If the piece can be used to clear more than 1 line, skip the entire piece.
                                } else {
                                    tmp_queue.push(w);
                                }
                            }
                        }
                    }

                    for well in tmp_queue {
                        if !queued_wells.contains_key(&(well.clone(), wev.0 .1 + 1)) {
                            queued_wells.insert((well.clone(), wev.0 .1 + 1), wev.1.clone());

                            let mut tmp_conf = conf.clone();
                            tmp_conf.quiescent = false;

                            let h = network_heuristic_individual(&well, weight, &tmp_conf);
                            for &id in wev.1 {
                                heuristics[id].1 = heuristics[id].1.max(h);
                            }
                            heuristic_map.insert(well, h);
                        } else {
                            let mut affected_wells = queued_wells
                                .get(&(well.clone(), wev.0 .1 + 1))
                                .unwrap()
                                .clone();
                            let mut to_update = wev.1.clone();
                            affected_wells.append(&mut to_update);
                            queued_wells.insert((well.clone(), wev.0 .1 + 1), affected_wells);
                            let h = *heuristic_map.get(&well).unwrap();
                            for &id in wev.1 {
                                heuristics[id].1 = heuristics[id].1.max(h);
                            }
                        }
                    }

                    break 'piece;
                }
            }
            wells_to_evaluate.clear();
            wells_to_evaluate = queued_wells;

            play_len += 1;
        }

        // LOOP DETECTION

        let mut max_heuristic: f64 = -1.0;
        for (_s, h) in &heuristics {
            max_heuristic = max_heuristic.max(*h);
        }

        let mut has_loop = false;
        let mut j = p;
        let mut d = parents[j].1.depth;
        'outer: while d > 0 {
            d = parents[j].1.depth;
            let min_prev = parents[j].1.min_prev_heuristic;
            let curr = parents[j].1.heuristic;
            if min_prev > max_heuristic || curr > max_heuristic {
                break 'outer;
            } else {
                // If this is slow, then you have to do a binary search on the sorted heuristic values.
                // and you can't use the default rust binary search, so we probably have to roll our own
                // or use a crate I guess.

                let parent_state = parents[j].1.convert_state();
                for (s, h) in heuristics.iter() {
                    if *h == curr && parent_state == *s {
                        has_loop = true;
                        break 'outer;
                    }
                }
            }

            j = parents[j].1.parent_index;
        }

        if has_loop {
            let mut j = parents.len() - 1;
            let mut d = parents[j].1.depth;
            let mut loop_list = Vec::with_capacity(d + 1);
            while d > 0 {
                d = parents[j].1.depth;
                loop_list.push(parents[j].1.convert_state());
                j = parents[j].1.parent_index;
            }

            return (heuristics, vec![loop_list]);
        } else {
            return (heuristics, vec![]);
        }
    }

    return (vec![], vec![]);
}
