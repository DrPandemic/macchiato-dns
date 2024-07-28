use smartstring::alias::String;
use std::collections::HashMap;

#[derive(Debug)]
struct Node {
    pub children: HashMap<String, Node>,
}

impl Node {
    pub fn new() -> Node {
        Node {
            children: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key_parts: Vec<&str>, fresh: bool) {
        if key_parts.is_empty() {
            self.children.clear();
            return;
        }
        if !fresh && self.children.is_empty(){
            return;
        }

        let key = key_parts[0];

        if let Some(next) = self.children.get_mut(key) {
            next.insert(key_parts[1..].to_vec(), false);
        } else {
            let next = Node::new();
            self.children.insert(key.into(), next);
            self.children
                .get_mut(key)
                .unwrap()
                .insert(key_parts[1..].to_vec(), true);
        }
    }
}

#[derive(Debug)]
pub struct Tree {
    root: Node,
}

#[derive(PartialEq, Debug)]
enum Processing {
    Success,
    Failed,
    Running,
}

impl Tree {
    pub fn new() -> Tree {
        Tree { root: Node::new() }
    }

    pub fn contains(&self, key: &String) -> Option<String> {
        let mut rule: Vec<&str> = vec![];
        let result = Tree::split(key)
            .into_iter()
            .fold((Some(&self.root), Processing::Running), |acc, key_part| match acc {
                (_, Processing::Success) => acc,
                (_, Processing::Failed) => acc,
                (Some(node), Processing::Running) => {
                    if let Some(next) = node.children.get(key_part) {
                        rule.push(key_part);
                        if next.children.len() == 0 {
                            (None, Processing::Success)
                        } else {
                            (Some(next), Processing::Running)
                        }
                    } else {
                        (None, Processing::Failed)
                    }
                }
                _ => (None, Processing::Failed),
            })
            .1;

        if result == Processing::Success {
            rule.reverse();
            Some(rule.join(".").into())
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: &String) {
        if key.len() > 0 {
            self.root.insert(Tree::split(key), true);
        }
    }

    fn split(key: &String) -> Vec<&str> {
        let mut parts = key.split(".").collect::<Vec<&str>>();
        parts.reverse();
        parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_contains() {
        let mut tree = Tree::new();
        tree.insert(&String::from("imateapot.org"));
        tree.insert(&String::from("www.imateapot.org"));
        tree.insert(&String::from("www.imateapot.info"));

        assert_eq!(
            Some(String::from("imateapot.org")),
            tree.contains(&String::from("imateapot.org"))
        );
        assert_eq!(
            Some(String::from("imateapot.org")),
            tree.contains(&String::from("www.imateapot.org"))
        );
        assert_eq!(
            Some(String::from("imateapot.org")),
            tree.contains(&String::from("m.www.imateapot.org"))
        );
        assert_eq!(None, tree.contains(&String::from("imateapot.ca")));
        assert_eq!(
            Some(String::from("www.imateapot.info")),
            tree.contains(&String::from("www.imateapot.info"))
        );
        assert_eq!(
            Some(String::from("www.imateapot.info")),
            tree.contains(&String::from("m.www.imateapot.info"))
        );
        assert_eq!(None, tree.contains(&String::from("imateapot.info")));
        assert_eq!(None, tree.contains(&String::from("org")));
        assert_eq!(None, tree.contains(&String::from("com")));
    }
}
