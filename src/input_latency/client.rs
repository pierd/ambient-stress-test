use std::collections::HashMap;

use ambient_api::{entity::get_component, message::Target, prelude::*};

const TARGET: Target = Target::ServerUnreliable;
const FRAMES_PER_MESSAGE: usize = 1; // send message every X frames
const ROLLING_AVERAGE_BUFFER_SIZE: usize = 60;

fn local_now() -> u64 {
    epoch_time().as_millis() as u64
}

#[derive(Clone, Debug)]
struct RollingAverageBuffer<const N: usize, T> {
    buffer: [T; N],
    index: usize,
    sum: T,
}

impl<const N: usize, T> RollingAverageBuffer<N, T> {
    fn add(&mut self, item: T)
    where
        T: Copy
            + Default
            + std::ops::Add<Output = T>
            + std::ops::Sub<Output = T>
            + std::ops::Div<Output = T>,
    {
        self.sum = self.sum + item - self.buffer[self.index];
        self.buffer[self.index] = item;
        self.index = (self.index + 1) % N;
    }

    fn average(&self) -> T
    where
        T: Copy + std::ops::Div<Output = T>,
        usize: TryInto<T>,
        <usize as TryInto<T>>::Error: std::fmt::Debug,
    {
        self.sum / N.try_into().unwrap()
    }
}

impl<const N: usize, T> Default for RollingAverageBuffer<N, T>
where
    T: Copy + Default + std::ops::Mul<Output = T>,
    usize: TryInto<T>,
    <usize as TryInto<T>>::Error: std::fmt::Debug,
{
    fn default() -> Self {
        Self {
            buffer: [T::default(); N],
            index: 0,
            sum: T::default() * N.try_into().unwrap(),
        }
    }
}

#[main]
pub fn main() {
    let player_id = player::get_local();
    let start_time = local_now();

    let mut seq_num = 1;
    let mut sent_timestamps = HashMap::new();
    let mut latencies = HashMap::<u64, usize>::new();
    let mut average_latency = RollingAverageBuffer::<ROLLING_AVERAGE_BUFFER_SIZE, u64>::default();
    let mut last_seq_num_seen = 0;
    let mut last_timestamp_seen = 0;
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
                    average_latency.add(latency);
                    if last_seq_num_seen < seq_num {
                        last_seq_num_seen = seq_num;
                    }
                    if last_timestamp_seen < sent_timestamp {
                        last_timestamp_seen = sent_timestamp;
                    }
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
            "Last {} messages average RTT: {}ms",
            ROLLING_AVERAGE_BUFFER_SIZE,
            average_latency.average()
        );
        println!(
            "Missing: {}/{}",
            sent_timestamps.len(),
            sent_timestamps.len() + responded_pings_count
        );
        println!(
            "Sequence gap sum (from server): {}",
            get_component(player_id, components::player_input_seq_skip()).unwrap_or(0)
        );
        println!("Current sequence gap: {}", seq_num - last_seq_num_seen);
        println!(
            "Current world latency: {}ms",
            timestamp - last_timestamp_seen
        );
        println!(
            "Average FPS: {}",
            frame_count as f64 / (timestamp - start_time) as f64 * 1000.0
        );
    });
}
