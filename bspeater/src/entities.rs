use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Entity {
    pub classname: String,
    pub keyvalues: HashMap<String, String>,
}

pub fn load_entitess(entities: Vec<u8>) -> Vec<Entity> {
    let Ok(entities) = String::from_utf8(entities) else {
        return Vec::new();
    };

    entities
        .split('{')
        .filter_map(parse_entity)
        .collect::<Vec<_>>()
}

fn parse_entity(ent: &str) -> Option<Entity> {
    let mut keyvalues = ent
        .trim()
        .trim_matches(['}', '{'])
        .trim()
        .lines()
        .filter_map(parse_keyvalue)
        .collect::<HashMap<String, String>>();

    Some(Entity {
        classname: keyvalues.remove("classname")?,
        keyvalues,
    })
}

fn parse_keyvalue(keyvalue: &str) -> Option<(String, String)> {
    let len = keyvalue.len();
    let mut keyvalue = keyvalue.trim().chars();

    // find first "
    while keyvalue.next()? != '"' {}

    let mut key = String::with_capacity(len / 3);

    // find second "
    while let Some(c) = keyvalue.next()
        && c != '"'
    {
        key.push(c);
    }

    // find third "
    while keyvalue.next()? != '"' {}

    let mut value = String::with_capacity(len / 3);

    // find fourth "
    while let Some(c) = keyvalue.next()
        && c != '"'
    {
        value.push(c);
    }

    Some((key, value))
}

impl Entity {
    pub fn get_model_index(&self) -> Option<usize> {
        self.keyvalues.get("model")?.get(1..)?.parse::<usize>().ok()
    }
}
