use super::*;

#[test]
fn parse1() {
    let ballast = Ballast::new();
    let mut arena = Arena::new(&ballast);
    Func::parse(&mut arena, r#"
block A {
   X <- Y;
   goto B C;
}
"#).unwrap();
}
