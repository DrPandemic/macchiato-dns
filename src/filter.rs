pub struct Filter {

}

const forbidden: [&str; 3] = ["avatars3", "githubusercontent", "com"];

impl Filter {
    pub fn is_filtered(&self, name: Vec<String>) -> bool {
        name[..] == forbidden
    }
}
