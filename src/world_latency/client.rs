use ambient_api::{message::Target, prelude::*};

const TARGET: Target = Target::ServerUnreliable;

#[main]
pub fn main() {
    let player_id = player::get_local();

    query(components::server_frame()).each_frame(|frame_counters| {
        let frame = frame_counters
            .first()
            .map(|(_, frame)| *frame)
            .unwrap_or_default();
        messages::FrameSeen { frame }.send(TARGET);
    });

    query(components::player_last_frame()).each_frame(move |player_frames| {
        let most_recent_frame = player_frames
            .iter()
            .map(|(_, frame)| *frame)
            .max()
            .unwrap_or_default();

        let mut message = String::new();
        for (id, frame) in player_frames {
            if !message.is_empty() {
                message.push_str(", ");
            }
            message.push_str(&format!("{}", most_recent_frame - frame));
            if id == player_id {
                message.push_str(" (you)");
            }
        }

        println!("Player frames: [{}]", message);
    });
}
