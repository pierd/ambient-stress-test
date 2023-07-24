use ambient_api::{components::core::player::player, entity::mutate_component, prelude::*};

const PING_COUNT: usize = 1024;

fn init_player_components(id: EntityId) {
    entity::add_component_if_required(
        id,
        components::player_input_last_idx(),
        PING_COUNT as u64 - 1,
    );
    entity::add_component_if_required(id, components::player_input_seq_nums(), vec![0; PING_COUNT]);
    entity::add_component_if_required(id, components::player_input_seq_skip(), 0);
}

#[main]
pub fn main() {
    spawn_query(player()).bind(|results| {
        for (id, _) in results {
            init_player_components(id);
        }
    });

    messages::Input::subscribe(move |source, msg| {
        println!("Received input message: {:?}", msg);

        let Some(player_entity_id) = source.client_entity_id() else {
            eprintln!("Received input message from unknown client");
            return;
        };
        let Some(idx) = mutate_component(
            player_entity_id,
            components::player_input_last_idx(),
            |last_idx| *last_idx = (*last_idx + 1) % PING_COUNT as u64,
        ) else {
            eprintln!("Received input message from client with no last_idx");
            return;
        };
        let idx = idx as usize;
        let prev_idx = if idx == 0 { PING_COUNT - 1 } else { idx - 1 };
        let mut seq_gap = 0;
        mutate_component(
            player_entity_id,
            components::player_input_seq_nums(),
            |seq_nums| {
                seq_nums[idx] = msg.seq_num;

                // check if there was a gap in the sequence numbers
                let prev_seq_num = seq_nums[prev_idx];
                if msg.seq_num < prev_seq_num {
                    eprint!("Out of order messages!");
                } else if msg.seq_num == prev_seq_num {
                    eprint!("Duplicate message!");
                } else if prev_seq_num != 0 && msg.seq_num != 0 {
                    assert!(msg.seq_num > prev_seq_num); // this should be guaranteed by the previous checks
                    seq_gap = (msg.seq_num - prev_seq_num) - 1;
                }
            },
        );
        if seq_gap > 0 {
            eprintln!("Sequence gap: {}!", seq_gap);
        }
        mutate_component(
            player_entity_id,
            components::player_input_seq_skip(),
            |seq_skip| *seq_skip += seq_gap,
        );
    });
}
