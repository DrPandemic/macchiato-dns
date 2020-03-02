use std::collections::HashMap;

#[derive(Debug)]
struct Node {
    pub key: Option<String>,
    pub children: HashMap<String, Node>,
}

impl Node {
    pub fn new(key: Option<String>) -> Node {
        Node {
            key: key,
            children: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key_parts: Vec<&str>) {
        if key_parts.len() == 0 {
            return;
        }
        let key = key_parts[0];

        self.children
            .entry(key.to_string())
            .or_insert(Node::new(Some(key.to_string())))
            .insert(key_parts[1..].to_vec());
    }
}

#[derive(Debug)]
pub struct Tree {
    root: Node,
}

impl Tree {
    pub fn new() -> Tree {
        Tree {
            root: Node::new(None),
        }
    }

    pub fn contains(&self, key: &String) -> bool {
        Tree::split(key)
            .into_iter()
            .fold(Some(&self.root), |node, key_part| {
                node.and_then(|node| node.children.get(key_part))
            })
            .is_some()
    }

    pub fn insert(&mut self, key: &String) {
        if key.len() > 0 {
            self.root.insert(Tree::split(key));
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
        tree.insert(&String::from("www.imateapot.info"));
        println!("{:?}", tree);
        assert_eq!(true, tree.contains(&String::from("imateapot.org")));
        assert_ne!(true, tree.contains(&String::from("imateapot.ca")));
        assert_eq!(true, tree.contains(&String::from("www.imateapot.info")));
        assert_ne!(true, tree.contains(&String::from("m.www.imateapot.info")));
    }
}
