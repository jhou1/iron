use ironcli::model::{PracticeType, SetData};

#[test]
fn practice_type_from_str() {
    assert_eq!("weighted".parse::<PracticeType>().unwrap(), PracticeType::Weighted);
    assert_eq!("bodyweight".parse::<PracticeType>().unwrap(), PracticeType::Bodyweight);
    assert_eq!("distance".parse::<PracticeType>().unwrap(), PracticeType::Distance);
    assert_eq!("endurance".parse::<PracticeType>().unwrap(), PracticeType::Endurance);
    assert!("invalid".parse::<PracticeType>().is_err());
}

#[test]
fn practice_type_display() {
    assert_eq!(PracticeType::Weighted.to_string(), "weighted");
    assert_eq!(PracticeType::Bodyweight.to_string(), "bodyweight");
    assert_eq!(PracticeType::Distance.to_string(), "distance");
    assert_eq!(PracticeType::Endurance.to_string(), "endurance");
}

#[test]
fn set_data_metric_weighted() {
    let set = SetData::Weighted { weight: 24.0, reps: 10 };
    assert_eq!(set.metric_value(), 240.0);
    assert_eq!(set.metric_label(), "kg vol");
}

#[test]
fn set_data_metric_bodyweight() {
    let set = SetData::Bodyweight { reps: 20 };
    assert_eq!(set.metric_value(), 20.0);
    assert_eq!(set.metric_label(), "reps");
}

#[test]
fn set_data_metric_distance() {
    let set = SetData::Distance { distance: 5.0 };
    assert_eq!(set.metric_value(), 5.0);
    assert_eq!(set.metric_label(), "km");
}

#[test]
fn set_data_metric_endurance() {
    let set = SetData::Endurance { duration: 30.0 };
    assert_eq!(set.metric_value(), 30.0);
    assert_eq!(set.metric_label(), "min");
}
