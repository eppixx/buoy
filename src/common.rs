pub fn convert_for_label(time: i64) -> String {
    //TODO error handling
    let time = chrono::TimeDelta::try_milliseconds(time).unwrap();
    let hours = time.num_hours();
    let minutes = (time - chrono::TimeDelta::try_hours(hours).unwrap()).num_minutes();
    let seconds = (time
        - chrono::TimeDelta::try_hours(hours).unwrap()
        - chrono::TimeDelta::try_minutes(minutes).unwrap())
    .num_seconds();

    let mut result = String::new();
    if hours > 0 {
        result.push_str(format!("{}:{:0>2}:{:0>2}", hours, minutes, seconds).as_str());
        return result;
    }
    result.push_str(format!("{}:{:0>2}", minutes, seconds).as_str());
    result
}

#[cfg(test)]
mod tests {
    use super::convert_for_label;

    #[test]
    fn convert_time() {
        let oracle = vec![
            (1000, "0:01"),
            (10000, "0:10"),
            (1000 * 60, "1:00"),
            (1000 * 60 * 10, "10:00"),
            (1000 * 60 * 60, "1:00:00"),
            (1000 * 60 * 60 * 10, "10:00:00"),
            (1000 * 60 * 60 + 1000, "1:00:01"),
        ];
        for test in oracle {
            assert_eq!(&convert_for_label(test.0), test.1);
        }
    }
}
