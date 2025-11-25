use std::{cell::RefCell, rc::Rc};

use relm4::gtk::{
    self,
    prelude::{AdjustmentExt, WidgetExt},
};

static UID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum AutomaticScrolling {
    #[default]
    Ready,
    GracePeriod,
    Scrolling,
}

pub trait ScrolledWindowExt {
    fn scroll_to(
        &self,
        scroll_to_percent: f64,
        status: Option<(std::time::Duration, Rc<RefCell<AutomaticScrolling>>)>,
    );
    fn smooth_scroll_to(
        &self,
        scroll_to_percent: f64,
        time: std::time::Duration,
        updates: std::time::Duration,
        status: Option<(std::time::Duration, Rc<RefCell<AutomaticScrolling>>)>,
    );
}

impl ScrolledWindowExt for gtk::ScrolledWindow {
    fn scroll_to(
        &self,
        scroll_to_percent: f64,
        status: Option<(std::time::Duration, Rc<RefCell<AutomaticScrolling>>)>,
    ) {
        if let Some((_, status)) = &status {
            status.replace(AutomaticScrolling::Scrolling);
        }
        let adj = self.vadjustment();
        let scroll_to_y = adj.upper() * scroll_to_percent;
        adj.set_value(scroll_to_y);
        self.set_vadjustment(Some(&adj));
        gtk::glib::spawn_future_local(async move {
            if let Some((grace, status)) = &status {
                status.replace(AutomaticScrolling::GracePeriod);
                tokio::time::sleep(*grace).await;
                status.replace(AutomaticScrolling::Ready);
            }
        });
    }

    fn smooth_scroll_to(
        &self,
        scroll_to_percent: f64,
        animation_length: std::time::Duration,
        update_delta: std::time::Duration,
        status: Option<(std::time::Duration, Rc<RefCell<AutomaticScrolling>>)>,
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

        // spawn thread that updates the scrolling
        let scroll = self.clone();
        let mut updates_done = 0.0;
        gtk::glib::spawn_future_local(async move {
            // set status to scrolling
            if let Some((_, status)) = &status {
                status.replace(AutomaticScrolling::Scrolling);
            }

            loop {
                tokio::time::sleep(update_delta).await;

                // check if another animation has started
                if let Ok(widget_id) = scroll.widget_name().parse::<usize>() {
                    if widget_id != uid {
                        // note: if another scroll interrupts this one, status is still scrolling
                        return;
                    }
                }

                // do the animation frame
                if updates_done <= total_updates {
                    // scrolling ended externally
                    if let Some((_, status)) = &status {
                        if *status.borrow() == AutomaticScrolling::Ready {
                            scroll.set_widget_name("");
                            return;
                        }
                    }

                    // do the scrolling
                    let new_value = keyframe::ease(
                        keyframe::functions::EaseInOutQuart,
                        start_value,
                        target_value,
                        updates_done / total_updates,
                    );
                    adjustment.set_value(new_value.floor());
                    updates_done += 1.0;
                    continue;
                } else {
                    // set target value
                    adjustment.set_value(target_value);
                    if let Some((grace, status)) = &status {
                        // wait the grace period for the next update
                        status.replace(AutomaticScrolling::GracePeriod);
                        tokio::time::sleep(*grace).await;
                        status.replace(AutomaticScrolling::Ready);
                    }

                    // reset widget
                    scroll.set_widget_name("");
                    break;
                }
            }
        });
    }
}
