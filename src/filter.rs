pub struct Filter {

}

impl Filter {
    fn is_filtered(name: Vec<String>) -> bool {
        match name {
            ["avatars3", "githubusercontent", "com"] => true,
            _ => false
        }
    }
}
