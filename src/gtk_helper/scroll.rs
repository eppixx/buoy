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
        let uid = UID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        if animation_length.is_zero() {
            panic!("given length is 0");
        }
        if update_delta.is_zero() {
            panic!("given updates_delta is 0");
        }

        // set current uid as widget_name
        // TODO find a nicer way to store if there is a new animation trigger
        self.set_widget_name(&uid.to_string());

        let adjustment = self.vadjustment();
        if adjustment.upper() <= 0.0 {
            return;
        }
        let start_value = adjustment.value();

        // calc start of widget; this scroll so the current played is at the top
        let target_value = adjustment.upper() * scroll_to_percent;

        // calc the steps needed
        let total_updates = animation_length.as_secs_f64() / update_delta.as_secs_f64();
        // calc the scrol_value increments for every step
        let step_value = (target_value - start_value) / total_updates;

        // spawn thread that updates the scrolling
        let scroll = self.clone();
        let mut updates_done = 0.0;
        gtk::glib::spawn_future_local(async move {
            loop {
                tokio::time::sleep(update_delta).await;

                // check if another animation has started
                if let Ok(widget_id) = scroll.widget_name().parse::<usize>() {
                    if widget_id != uid {
                        return;
                    }
                }

                // do the animation frame
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
