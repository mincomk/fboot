use crate::app_state::AppState;

pub mod arp_task;
pub mod stats_task;
pub mod status_task;

pub fn spawn_all(state: AppState) {
    arp_task::spawn(state.clone());
    status_task::spawn(state.clone());
    stats_task::spawn(state);
}
