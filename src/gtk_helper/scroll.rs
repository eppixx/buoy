use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use futures::lock::Mutex;
use gstreamer::glib::object::ObjectExt;
use relm4::gtk::{
    self,
    prelude::{AdjustmentExt, WidgetExt},
};

static UID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
// static scrolled_ids: Arc<Mutex<HashMap<usize, usize>>> = Arc::new(Mutex::new(HashMap::new()));
//TODO interrupt previous scrolling

pub trait ScrolledWindowExt {
    fn scroll_to(&self, scroll_to_percent: f64);
    fn smooth_scroll_to(
        &self,
        scroll_to_percent: f64,
        time: std::time::Duration,
        updates: std::time::Duration,
    );
}

impl ScrolledWindowExt for gtk::ScrolledWindow {
    fn scroll_to(&self, scroll_to_percent: f64) {
        let adj = self.vadjustment();
        let scroll_to_y = adj.upper() * scroll_to_percent;
        adj.set_value(scroll_to_y);
        self.set_vadjustment(Some(&adj));
    }

    fn smooth_scroll_to(
        &self,
        scroll_to_percent: f64,
        animation_length: std::time::Duration,
        update_delta: std::time::Duration,
    ) {
        if animation_length.is_zero() {
            panic!("given length is 0");
        }
        if update_delta.is_zero() {
            panic!("given updates_delta is 0");
        }

        let adjustment = self.vadjustment();
        if adjustment.upper() <= 0.0 {
            return;
        }
        let start_value = adjustment.value();

        // calc start of widget; this scroll so the current played is at the top
        let target_value = adjustment.upper() * scroll_to_percent;
        // scroll, so that the played is in the middle of widget
        let target_value = target_value - f64::from(self.height()) * 0.45;

        // calc the steps needed
        let total_updates = (animation_length.as_secs_f64() / update_delta.as_secs_f64()) as f64;
        // calc the scrol_value increments for every step
        let step_value = (target_value - start_value) / total_updates;

        // spawn thread that updates the scrolling
        let mut updates_done = 0.0;
        gtk::glib::spawn_future_local(async move {
            loop {
                tokio::time::sleep(update_delta).await;

                if updates_done <= total_updates {
                    let new_value = start_value + step_value * updates_done;
                    adjustment.set_value(new_value);
                    updates_done += 1.0;
                    continue;
                } else {
                    adjustment.set_value(target_value);
                    break;
                }
            }
        });
    }
}
