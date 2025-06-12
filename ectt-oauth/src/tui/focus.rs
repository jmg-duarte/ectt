pub trait FocusStyle {
    fn focused(&mut self);

    fn unfocused(&mut self);
}
