use std;
use std::io::Read;
use std::io::Write;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use std::string::ToString;
use std::convert::From;

use base64;
use json;
use json::JsonValue;

pub trait Config {
    fn get(&self, key: &str) -> Option<Value>;
    fn set<T: Into<Value>>(&mut self, key: &str, value: T);
    fn delete(&mut self, key: &str);

    fn get_str(&self, key: &str) -> Option<String> {
        self.get(key).and_then(|v| match v {
            Value::String(x) => Some(x),
            _ => None,
        })
    }
    fn get_f64(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| match v {
            Value::Number(x) => Some(x),
            _ => None,
        })
    }
    fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| match v {
            Value::Bool(x) => Some(x),
            _ => None,
        })
    }
    fn get_bytes(&self, key: &str) -> Option<Vec<u8>> {
        self.get(key).and_then(|v| match v {
            Value::Bytes(x) => Some(x),
            _ => None,
        })
    }
}

#[derive(Debug)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Bytes(Vec<u8>),
    Null,
}

impl From<u64> for Value {
    fn from(x: u64) -> Self {
        Value::Number(x as f64)
    }
}

impl From<i64> for Value {
    fn from(x: i64) -> Self {
        Value::Number(x as f64)
    }
}

impl From<f64> for Value {
    fn from(x: f64) -> Self {
        Value::Number(x)
    }
}

impl From<bool> for Value {
    fn from(x: bool) -> Self {
        Value::Bool(x)
    }
}

impl<'a> From<&'a str> for Value {
    fn from(x: &str) -> Value {
        Value::String(x.to_string())
    }
}

impl From<String> for Value {
    fn from(x: String) -> Value {
        Value::String(x)
    }
}

impl<'a> From<&'a String> for Value {
    fn from(x: &String) -> Value {
        Value::String(x.clone())
    }
}

impl<'a> From<&'a [u8]> for Value {
    fn from(x: &[u8]) -> Value {
        Value::Bytes(x.to_vec())
    }
}

pub struct JsonConfig {
    auto_save: bool,
    path: PathBuf,
    data: JsonValue,
}

#[derive(Debug)]
pub enum JsonConfigError {
    Parse(json::Error),
    Io(std::io::Error),
}

const BASE64_PREFIX: &'static str = "base64:";

impl JsonConfig {
    pub fn new<T: AsRef<Path>>(path: T, auto_save: bool) -> Self {
        JsonConfig {
            auto_save: auto_save,
            path: path.as_ref().to_path_buf(),
            data: JsonValue::new_object(),
        }
    }

    pub fn load(&mut self) -> Result<(), JsonConfigError> {
        let mut json_string = String::new();
        match File::open(&self.path).and_then(|mut file| file.read_to_string(&mut json_string)) {
            Ok(_) => (),
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::NotFound => return Ok(()),
                    _ => return Err(JsonConfigError::Io(e)),
                }
            }
        };
        json::parse(&json_string)
            .map_err(JsonConfigError::Parse)
            .and_then(|v| {
                if v.is_object() {
                    self.data = v;
                    Ok(())
                } else {
                    Err(JsonConfigError::Parse(json::Error::WrongType("not object".to_string())))
                }
            })
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        File::create(&self.path).and_then(|mut file| {
            let str = self.data.pretty(4);
            file.write_all(str.as_bytes())
        })
    }

    fn lookup(&self, key: &str) -> &JsonValue {
        let keys: Vec<&str> = key.split('.').collect();
        Self::_lookup(&self.data, &keys)
    }

    fn _lookup<'a>(node: &'a JsonValue, keys: &[&str]) -> &'a JsonValue {
        let (name, keys2) = keys.split_first().unwrap();
        if keys2.is_empty() {
            &node[*name]
        } else {
            Self::_lookup(&node[*name], keys2)
        }
    }

    fn lookup_mut(&mut self, key: &str) -> &mut JsonValue {
        let keys: Vec<&str> = key.split('.').collect();
        &mut Self::_lookup_parent(&mut self.data, &keys)[*keys.last().unwrap()]
    }

    fn _lookup_parent<'a>(node: &'a mut JsonValue, keys: &[&str]) -> &'a mut JsonValue {
        let (name, keys2) = keys.split_first().unwrap();
        if keys2.is_empty() {
            node
        } else {
            Self::_lookup_parent(&mut node[*name], keys2)
        }
    }

    fn str_to_value(x: String) -> Value {
        if x.starts_with(BASE64_PREFIX) {
            if let Ok(x) = base64::decode(&x[BASE64_PREFIX.len()..]) {
                return Value::Bytes(x);
            }
        }
        Value::String(x)
    }
}

impl Config for JsonConfig {
    fn get(&self, key: &str) -> Option<Value> {
        match *self.lookup(key) {
            JsonValue::Short(ref x) => Some(Self::str_to_value(x.to_string())),
            JsonValue::String(ref x) => Some(Self::str_to_value(x.to_string())),
            JsonValue::Number(ref x) => Some(Value::Number(f64::from(*x))),
            JsonValue::Boolean(ref x) => Some(Value::Bool(*x)),
            _ => None,
        }
    }

    fn set<T: Into<Value>>(&mut self, key: &str, value: T) {
        *self.lookup_mut(key) = match value.into() {
            Value::String(x) => JsonValue::String(x),
            Value::Number(x) => JsonValue::Number(json::number::Number::from(x)),
            Value::Bool(x) => JsonValue::Boolean(x),
            Value::Bytes(x) => JsonValue::String(BASE64_PREFIX.to_string() + &base64::encode(&x)),
            Value::Null => JsonValue::Null,
        };
        if self.auto_save {
            self.save().unwrap();
        }
    }

    fn delete(&mut self, key: &str) {
        let keys: Vec<&str> = key.split('.').collect();
        Self::_lookup_parent(&mut self.data, &keys).remove(*keys.last().unwrap());
        if self.auto_save {
            self.save().unwrap();
        }
    }
}

#[cfg(target_os="linux")]
pub fn get_config_dir_path() -> PathBuf {
    let mut path = match std::env::var("XDG_CONFIG_HOME") {
        Ok(x) => PathBuf::from(x),
        _ => {
            let mut x = PathBuf::from(std::env::home_dir().unwrap());
            x.push(".config");
            x
        }
    };
    path.push(env!("CARGO_PKG_NAME"));
    path
}

#[test]
fn test() {
    let path = "test.json";
    {
        let mut config = JsonConfig::new(path, false);
        config.set("foo", "value");
        config.set("hoge.piyo", "test");
        config.set("hoge.pi", std::f64::consts::PI);
        config.set("hoge.flag0", true);
        config.set("hoge.flag1", false);
        config.set("hoge.raw", b"Hello World" as &[u8]);
        assert_eq!("value", config.get_str("foo").unwrap());
        assert_eq!("test", config.get_str("hoge.piyo").unwrap());
        assert_eq!(std::f64::consts::PI, config.get_f64("hoge.pi").unwrap());
        assert_eq!(true, config.get_bool("hoge.flag0").unwrap());
        assert_eq!(false, config.get_bool("hoge.flag1").unwrap());
        assert_eq!(b"Hello World".to_vec(),
                   config.get_bytes("hoge.raw").unwrap());
        assert!(config.get("foobar").is_none());
        config.save().unwrap();
    }
    {
        let mut config = JsonConfig::new(path, true);
        config.load().unwrap();
        assert_eq!("value", config.get_str("foo").unwrap());
        assert_eq!("test", config.get_str("hoge.piyo").unwrap());
        assert_eq!(std::f64::consts::PI, config.get_f64("hoge.pi").unwrap());
        assert_eq!(true, config.get_bool("hoge.flag0").unwrap());
        assert_eq!(false, config.get_bool("hoge.flag1").unwrap());
        assert!(config.get("foobar").is_none());
        config.set("hoge.piyo", "bar");
        config.set("test", "helloworld");
    }
    {
        let mut config = JsonConfig::new(path, true);
        config.load().unwrap();
        assert_eq!("value", config.get_str("foo").unwrap());
        assert_eq!("bar", config.get_str("hoge.piyo").unwrap());
        assert_eq!("helloworld", config.get_str("test").unwrap());
        config.delete("test");
        assert!(config.get("test").is_none());
    }
    {
        assert!(JsonConfig::new("notfound.json", false).load().is_ok());
        std::fs::File::create(path)
            .and_then(|mut f| f.write_all(b"{\"broken\": \"json"))
            .unwrap();
        assert!(JsonConfig::new(path, false).load().is_err());
    }
    std::fs::remove_file(path).unwrap();
}
