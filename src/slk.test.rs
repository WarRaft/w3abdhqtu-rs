use super::*;

#[test]
fn cells_by_row_and_column() {
    let text = "ID;PWXL;N;E\r\nB;X3;Y3;D0\r\nC;X1;Y1;K\"ID\"\r\nC;X3;K\"name\"\r\nF;P0;X1;Y2\r\nC;K12\r\nC;X2;K\"a;;b\"\r\nC;Y3;X3;K\"\"\r\nE\r\n";
    assert_eq!(
        table(text),
        vec![
            vec!["ID".to_string(), String::new(), "name".into()],
            vec!["12".into(), "a;b".into()],
            vec![String::new(), String::new(), String::new()],
        ]
    );
}

#[test]
fn nothing_to_read() {
    assert!(table("").is_empty());
    assert!(table("ID;P\nB;X1;Y1\nE\n").is_empty());
    // Row 0 doesn't exist.
    assert!(table("C;X1;Y0;K1\n").is_empty());
}
