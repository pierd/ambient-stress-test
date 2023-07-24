use std::collections::HashMap;

use ambient_api::{entity::get_component, message::Target, prelude::*};

const TARGET: Target = Target::ServerReliable;
const FRAMES_PER_MESSAGE: usize = 4; // send message every X frames

fn local_now() -> u64 {
    epoch_time().as_millis() as u64
}

#[main]
pub fn main() {
    let player_id = player::get_local();

    let mut seq_num = 1;
    let mut sent_timestamps = HashMap::new();
    let mut latencies = HashMap::<u64, usize>::new();
    let mut frame_count = 0;

    ambient_api::messages::Frame::subscribe(move |_| {
        frame_count += 1;

        let timestamp = local_now();

        if frame_count % FRAMES_PER_MESSAGE == 0 {
            messages::Input { seq_num }.send(TARGET);
            sent_timestamps.insert(seq_num, timestamp);
            seq_num += 1;
        }

        if let Some(seq_nums) = get_component(player_id, components::player_input_seq_nums()) {
            for seq_num in seq_nums.into_iter() {
                if let Some(sent_timestamp) = sent_timestamps.remove(&seq_num) {
                    let latency = timestamp - sent_timestamp;
                    *latencies.entry(latency).or_default() += 1;
                }
            }
        }

        let responded_pings_count: usize = latencies.iter().map(|(_, count)| count).sum();
        if responded_pings_count > 0 {
            let average_rtt = latencies
                .iter()
                .map(|(latency, count)| *latency as usize * *count)
                .sum::<usize>()
                / responded_pings_count;
            println!("Average RTT: {}ms", average_rtt);
        }
        println!(
            "Missing: {}/{}",
            sent_timestamps.len(),
            sent_timestamps.len() + responded_pings_count
        );
        println!(
            "Sequence gap sum: {}",
            get_component(player_id, components::player_input_seq_skip()).unwrap_or(0)
        );
    });
}
