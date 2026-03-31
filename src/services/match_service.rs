use crate::models::MatchRow;

pub fn list_featured_matches() -> Vec<MatchRow> {
    vec![
        MatchRow {
            mat: "1".into(),
            category: "Senior Men - Single Stick".into(),
            red: "Fighter A".into(),
            blue: "Fighter B".into(),
            status: "LIVE".into(),
            status_class: "status-live".into(),
        },
        MatchRow {
            mat: "2".into(),
            category: "Junior - Double Stick".into(),
            red: "Fighter C".into(),
            blue: "Fighter D".into(),
            status: "READY".into(),
            status_class: "status-ready".into(),
        },
    ]
}
