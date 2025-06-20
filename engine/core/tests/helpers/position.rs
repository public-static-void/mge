use serde_json::Value;

/// Compare two Position components for equality by their "Square" coordinates.
pub fn pos_eq(a: &Value, b: &Value) -> bool {
    let a_sq = a.get("pos").and_then(|p| p.get("Square"));
    let b_sq = b.get("pos").and_then(|p| p.get("Square"));
    let (ax, ay, az) = (
        a_sq.and_then(|sq| sq.get("x")).and_then(|v| v.as_i64()),
        a_sq.and_then(|sq| sq.get("y")).and_then(|v| v.as_i64()),
        a_sq.and_then(|sq| sq.get("z")).and_then(|v| v.as_i64()),
    );
    let (bx, by, bz) = (
        b_sq.and_then(|sq| sq.get("x")).and_then(|v| v.as_i64()),
        b_sq.and_then(|sq| sq.get("y")).and_then(|v| v.as_i64()),
        b_sq.and_then(|sq| sq.get("z")).and_then(|v| v.as_i64()),
    );
    ax == bx && ay == by && az == bz
}
