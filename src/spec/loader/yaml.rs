extern crate yaml_rust;
use self::yaml_rust::YamlLoader;


#[cfg(test)]
mod test {
    extern crate yaml_rust;
    use self::yaml_rust::{YamlLoader, Yaml};
    use std::fs::File;
    use std::io::prelude::*;
    use std::collections::BTreeMap;
    use spec::{FieldSpec, RecordSpec, PaddingDirection};

    #[test]
    fn read_record() {
        let mut file = File::open("src/spec/loader/spec.yml").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        let docs = YamlLoader::load_from_str(&contents).unwrap();

        for doc in docs {
            let mut doc = get_hash(doc);

            let mut records = get_hash(doc.remove(&Yaml::String("records".to_string())).unwrap());

            for (name, spec) in records {
                let name = get_string(name);
                let mut spec = get_hash(spec);

                let mut fields = get_hash(spec.remove(&Yaml::String("fields".to_string())).unwrap());
                println!("{:?}", fields);

                for (field_name, field_spec) in fields {
                    let field_name = get_string(field_name);
                    let mut field_spec = get_hash(field_spec);

                    let field = FieldSpec {
                        default: field_spec.remove(&Yaml::String("default".to_string())).map(get_u8_vec),
                        length: field_spec.remove(&Yaml::String("length".to_string())).map(get_usize).unwrap(),
                        padding: field_spec.remove(&Yaml::String("padding".to_string())).map(get_u8_vec).unwrap_or_else(|| Vec::new()),
                        padding_direction: field_spec.remove(&Yaml::String("padding_direction".to_string())).map(get_padding_direction).unwrap()
                    };
                    println!("{:?}", field);
                }

            }
        }

    }

    fn get_hash(value: Yaml) -> BTreeMap<Yaml, Yaml> {
        match value {
            Yaml::Hash(v) => v,
            _ => panic!()
        }
    }

    fn get_string(value: Yaml) -> String {
        match value {
            Yaml::String(v) => v,
            _ => panic!()
        }
    }

    fn get_u8_vec(value: Yaml) -> Vec<u8> {
        println!("{:?}", value);
        match value {
            Yaml::String(v) => v.into_bytes(),
            Yaml::Integer(v) => v.to_string().into_bytes(),
            _ => panic!()
        }
    }

    fn get_usize(value: Yaml) -> usize {
        match value {
            Yaml::Integer(v) => v as usize,
            _ => panic!()
        }
    }

    fn get_padding_direction(value: Yaml) -> PaddingDirection {
        match value {
            Yaml::String(ref v) if v == "right" => PaddingDirection::Right,
            Yaml::String(ref v) if v == "left" => PaddingDirection::Left,
            _ => panic!()
        }
    }
}
