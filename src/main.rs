extern crate serde_derive;
extern crate yaml_rust;
extern crate serde;
extern crate reqwest;
extern crate regex;

use yaml_rust::{YamlLoader, Yaml};
use std::fs;
use std::fmt;
use std::error;
use serde::{Serialize};
use regex::RegexSet;

fn main() {
    let mut args = std::env::args();
    let mut owner = String::new();
    let mut api = String::new();
    let mut apiVersion = String::new();
    let mut authorization = String::new();
    let mut fileName = String::new();
    args.next();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--owner" | "-o"=> {
                owner = args.next().expect("An owner should be supplied");
            },
            "--api" | "-a" => {
                api = args.next().expect("An api should be given");
            },
            "--apiVersion" | "-v" => {
                apiVersion = args.next().expect("A version string is needed");
            },
             "--authorization" | "-s" => {
                authorization = args.next().expect("Authorization is needed to access private api");
            },
             "--file" | "-f" => {
                fileName = args.next().expect("Filename should be provided");
            },
             "--help" | "-h" => {
                 println!("usage: rusty_swagger --owner [API_OWNER] --api [API_NAME] --apiVersion [API_VERSION] --authorization [API_TOKEN]");
                 println!("rusty_swagger --file [PATH_TO_SWAGGER_DEFINITION]");
                std::process::exit(0);
            },
            _ => {
                panic!("We need three arguments");
            }
        }
    }
    if fileName.is_empty() {
        if owner.is_empty() || api.is_empty() || apiVersion.is_empty() {
            println!("We need either e filename or the swaggerhub options. Use the --help switch");
            std::process::exit(1);
        }
        get_swagger_config(&owner, &api, &apiVersion, &authorization);
        fileName = format!("swagger-{}.yaml", apiVersion);
    }
    let content = fs::read_to_string(fileName).expect("Could not read swagger yaml");

    let docs = YamlLoader::load_from_str(content.as_str()).unwrap();

    // Multi document support, doc is a yaml::Yaml
    let doc = &docs[0];
    let theDoc = Documentation::new(doc);
    let template = mustache::compile_path("documentation.tex.mustache").expect("could not find template");
    let output = match template.render_to_string(&theDoc) {
        Ok(content) => {
            content
        },
        Err(_) =>
        {
            panic!("could not render");
        }
    };
    std::fs::write("documentation.tex", output).expect("Could not write tex");
    //start filling our model
   // println!("{:?}", theDoc);
}
pub fn get_swagger_config(owner : &str, api : &str, version : &str, authorization : &str){
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Authorization", authorization.parse().unwrap());
    headers.insert("Accept", "application/yaml".parse().unwrap());
    let client = reqwest::Client::builder().default_headers(headers).gzip(true).build().expect("Could not build HttpClient");
    let body = client.get(&format!("https://api.swaggerhub.com/apis/{}/{}/{}", owner, api, version)).send().expect("Request sending failed").text().expect("failure retrieving body");
    println!("{}", version);
    std::fs::write(&format!("swagger-{}.yaml", version), body).expect("Could not write swagger definition");
}

//Structures and function to parse and fill the latex templates
#[derive(Serialize, Debug)]
struct Documentation {
    title : String,
    host : String,
    base_url : String,
    requests : Vec<Request>,
    schemas : Vec<Schema>
}

#[derive(Debug, Clone)]
struct TypeNotFound {

}
impl std::fmt::Display for TypeNotFound {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Could not find type")
    }
}
impl error::Error for TypeNotFound {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

fn get_typeName(schema : & Yaml) -> Result<String,TypeNotFound>{
    if let Yaml::String(val) = &schema["schema"]["type"] {
        match val.as_str() {
        "array" => {
            let innerType = if let Yaml::String(innerProp) = &schema["schema"]["items"]["type"]
            {
                String::from(innerProp.as_str())
            } else if let Yaml::String(innerProp) = &schema["schema"]["items"]["$ref"]{
                let types : Vec<&str> = val.split("/").collect();
                let theType = types.last().expect("no last element");
                String::from(shorten_type_name(theType))
            } else {
                String::from("unknown property")
            };
            Ok( String::from(format!("{}[]", innerType)))
        },
        "object" => {
            //we have an object, since every other case should be handled by a reference to a model
            let innerType = if let Yaml::String(addProp) = &schema["additionalProperties"]["$ref"] {
                let types : Vec<&str> = val.split("/").collect();
                let theType = types.last().expect("no last element");
                String::from(shorten_type_name(theType))
            } else if let Yaml::String(addProp) = &schema["additionalProperties"]["type"] {
                String::from(addProp.as_str())
            } else {
                String::from("unknown type")
            };
            Ok( String::from(format!("Map<string,{}>", shorten_type_name(&innerType))))
        },
        _ => {
             Ok(String::from(val.as_str()))
        }
    }
    } else if let Yaml::String(val) = &schema["schema"]["$ref"] {
        let types : Vec<&str> = val.split("/").collect();
        let theType = types.last().expect("no last element");
        println!("{}",val);
       Ok(shorten_type_name(&theType))
    }else if  let Yaml::String(val) = &schema["type"] {
        Ok(String::from(val.as_str()))
    }
    else {
        Err(TypeNotFound {})
    }
}

fn shorten_type_name(long_type_name : &str) -> String{
    //we can shorten the name by spliting the name at dots and rejoin the paths only if the first letter is a capital
    if ! long_type_name.contains(".") {
        return String::from(long_type_name);
    }
    let class_parts = long_type_name.split(".");
    let mut class_name = String::new();
    for part in class_parts {
        let first_char = part.chars().next().unwrap();
        if  first_char.is_uppercase(){
            class_name.push_str(".");
            class_name.push_str(part);
        }
    };
    //skip the first character since it is a dot
    String::from(&class_name[1..])
}

fn get_type_name_without_schema(prop : &Yaml) -> Result<String,TypeNotFound> {
    if let Yaml::String(val) = &prop["type"] {
        match val.as_str() {
        "array" => {
            let innerType = if let Yaml::String(innerProp) = &prop["items"]["type"]
            {
               String::from(innerProp.as_str())
            } else if let Yaml::String(innerProp) = &prop["items"]["$ref"]{
                String::from( shorten_type_name(&(innerProp.as_str().replace("#/components/schemas/", ""))))
            } else {
                String::from("unknown property")
            };
            Ok( String::from(format!("{}[]", innerType)))
        },
        "object" => {
            //for responses we either have a map, or a field holding the array
            let innerType = if let Yaml::String(addProp) = &prop["additionalProperties"]["$ref"] {
                String::from(addProp.as_str().replace("#/components/schemas/", ""))
            } else if let Yaml::String(addProp) = &prop["additionalProperties"]["type"] {
                String::from(addProp.as_str())
            } else {
                String::from("unknown type")
            };
            Ok(String::from(format!("Map<string,{}>", shorten_type_name(&innerType))))
        },
        _ => {
            Ok(String::from(val.as_str()))
        }
    }
    } else if let Yaml::String(val) = &prop["$ref"] {
        Ok(shorten_type_name(&(val.as_str().replace("#/components/schemas/", ""))))
    } else {
        Err(TypeNotFound {})
    }
}

fn get_paths(paths : &Yaml) -> Vec<Request>{
if let Yaml::Hash(ref h) = paths {
            let mut result = Vec::new();
            for (k,v) in h {
                let mut headers = RequestHeader::new();
                let mut querys = QueryParameter::new();
                let mut bodys = ResponseBody::new();
                let mut responses = Vec::new();
                let mut descriptionString = String::new();
                let methods = if let Yaml::Hash(ref v) = paths[k.as_str().unwrap()] {
                    let mut arr = Vec::new();
                    for (method,details) in v {
                        let theMethod = method.as_str().unwrap();
                       
                        let meth = Method::new(String::from(theMethod), String::from(k.as_str().unwrap()), String::from(paths[k.as_str().unwrap()][theMethod]["summary"].as_str().unwrap_or("")),  markdown_to_latex(paths[k.as_str().unwrap()][theMethod]["description"].as_str().unwrap_or("")));
                        //println!("{}", meth.description);
                        descriptionString = String::from(meth.description.as_str());
                        arr.push(meth);
                        //loop over parameters
                        let params = &paths[k.as_str().unwrap()][theMethod]["parameters"];
                        if let Yaml::Array(ref parameter_list) = params {
                           for entry in parameter_list {
                               //println!("{:?}", entry);
                               let typeName = get_typeName(&entry).unwrap_or(String::from("unknown\\_type"));
                                let mut example = String::new();
                                        if let Yaml::String(val) = &entry["example"] {
                                            example = String::from(val.as_str());
                                        }
                                let param = Field::new(String::from(entry["name"].as_str().unwrap_or("").replace("_", "\\_")),typeName, entry["required"].as_bool().unwrap_or(false),markdown_to_latex(entry["description"].as_str().unwrap_or("")), example);
                               // println!("{}", param.description);
                                match entry["in"] {
                                    Yaml::String(ref val) if val.contains("query") => {
                                        querys.add(param);
                                    },
                                     Yaml::String(ref val) if val.contains("header") => {
                                        let mut example = String::new();
                                        if let Yaml::String(val) = &entry["example"] {
                                            example = String::from(val.as_str());
                                        }
                                        headers.add(param);
                                    },
                                    _ => {}
                                };
                           }
                        }
                        //add a request body if needed
                        if let Yaml::Hash(ref inner) = details["requestBody"] {
                            let typeName = get_typeName(&details["requestBody"]["content"]["application/json"]).unwrap_or(String::from("unknown type"));
                            let mut example = String::new();
                            if let Yaml::String(val) = &details["requestBody"]["example"] {
                                example = String::from(val.as_str());
                            }
                            let theField = Field::new(String::from(""), typeName, details["requestBody"]["required"].as_bool().unwrap_or(false),markdown_to_latex(details["requestBody"]["description"].as_str().unwrap_or("N/A")), example);
                            bodys.add(theField);
                        }
                        //add the response if needed
                        if let Yaml::Hash(ref inner) = &details["responses"] {
                            for (responseCode, responseNode) in inner {
                                let mut response = Response::new();
                                let mut typeName = String::from("");
                                let responseCodeString = responseCode.as_str().unwrap();
                                if !&details["responses"][responseCodeString]["content"]["application/json"].is_badvalue() {
                                    typeName = get_typeName(&details["responses"][responseCodeString]["content"]["application/json"]).unwrap_or(String::from("unkown type"));
                                } else if !&details["responses"][responseCodeString]["schema"].is_badvalue() {
                                    //we have a global defined content type
                                    typeName = get_typeName(&details["responses"][responseCodeString]).unwrap_or(String::from("unknown type"));
                                }
                                else {
                                    if let Yaml::Hash(innerMap) = &details["responses"][responseCodeString]["content"] {
                                        for (k,v) in innerMap {
                                            response.set_content_type(String::from(k.as_str().unwrap()));
                                            break;
                                        }
                                    }
                                
                                }
                                let fieldName = "*";
                                let required = if let Yaml::Boolean(val) = &details["requestBody"]["required"] {
                                    *val
                                } else {
                                    false
                                };
                                let mut example = String::new();
                                if let Yaml::String(val) = &details["responses"][responseCodeString]["example"] {
                                    example = String::from(val.as_str());
                                }
                                let theField = Field::new(String::from(""), typeName, required,markdown_to_latex(details["responses"][responseCodeString]["description"].as_str().unwrap()), example);
                                response.set_description(markdown_to_latex(details["responses"][responseCodeString]["description"].as_str().unwrap()));
                                response.set_status_code(responseCodeString.to_string());
                                response.add(theField);
                                responses.push(response);
                            }
                        }
                    }
                    arr
                }
                else {
                    Vec::new()
                };
            
                result.push(Request::new(String::from(k.as_str().unwrap()), methods, descriptionString, if headers.required() {Option::Some(headers)} else { Option::None}, if querys.required() {Option::Some(querys)} else { Option::None}, if bodys.required() {Option::Some(bodys)} else { Option::None}, responses));
            }
            result
        } else {
            panic!("Could not find paths");
        }
}

impl Documentation {
    pub fn new(documentRoot : &Yaml) -> Documentation{
        let title = documentRoot["info"]["title"].as_str().unwrap();
        let host = documentRoot["servers"][0]["url"].as_str().unwrap_or("-");
        
        let requests = get_paths(&documentRoot["paths"]);
        let rootModels = if documentRoot["components"]["schemas"].is_badvalue() {
            &documentRoot["definitions"]
        } else {
            &documentRoot["components"]["schemas"]
        };
        let schemas = if let Yaml::Hash(ref h) = rootModels {
            let mut result = Vec::new();
            for (k,v) in h {
                let mut properties = Vec::new();
                let mut enumValues = Vec::new();
                let mut required = Vec::new();
                if let Yaml::Array(ref required_properties) = documentRoot["components"]["schemas"][k.as_str().unwrap()]["required"] {
                    for required_property in required_properties {
                        required.push(required_property.as_str().unwrap());
                       // println!("{:?}", required_property);
                    }
                }
                if let Yaml::Hash(ref inner) = documentRoot["components"]["schemas"][k.as_str().unwrap()]["properties"] {
                    for (propName, propNode) in inner {
                        let fieldName = shorten_type_name(&propName.as_str().unwrap().replace("_", "\\_"));
                        let type_name =  get_type_name_without_schema(&propNode).unwrap_or(String::from("unknown type"));
                        let is_property_required = required.contains(&propName.as_str().unwrap());
                        let mut description = String::from("");
                        let mut example = String::from("");
                        if let Yaml::String(val) = &propNode["description"] {
                            description = markdown_to_latex(val.as_str());
                        }
                        if let Yaml::String(val) = &propNode["example"] {
                            example = String::from(val.as_str());
                        }
                        let theField = Field::new(fieldName,type_name,is_property_required,description, example);
                        properties.push(theField);
                    }
                } else if let Yaml::Array(ref inner) = documentRoot["components"]["schemas"][k.as_str().unwrap()]["enum"] {
                    for enumName in inner {
                        enumValues.push(Field::new(String::from(enumName.as_str().unwrap().replace("_","\\_")), String::from(""), false, String::from(""), String::from("")));
                    }
                }
                result.push(Schema::new(shorten_type_name(k.as_str().unwrap()), properties, enumValues));
            }
            result
        } else {
            panic!("Could not find paths");
        };
        Documentation {
            title : String::from(title),
            host : String::from(host),
            base_url : String::from(""),
            requests: requests,
            schemas : schemas
        }
    }
}

#[derive(Serialize, Debug)]
struct Request {
    title : String,
    methods : Vec<Method>,
    description : String,
    request_headers: Option<RequestHeader>,
    query_parameters: Option<QueryParameter>,
    response_body: Option<ResponseBody>,
    responses : Vec<Response>
}
impl Request {
    pub fn new(title : String, methods : Vec<Method>, description : String, request_headers : Option<RequestHeader>, query_parameters : Option<QueryParameter>, response_body : Option<ResponseBody>, responses : Vec<Response>) -> Request {
        Request {
            title,
            methods,
            description,
            request_headers,
            query_parameters,
            response_body,
            responses
        }
    }
}
#[derive(Serialize, Debug)]
struct Method {
    method : String,
    path : String,
    summary : String,
    description : String
}
impl Method {
    pub fn new(method : String, path:String, summary : String, description : String) -> Method{
        Method {
            method,
            path,
            summary,
            description
        }
    }
}
#[derive(Serialize, Debug)]
struct RequestHeader {
    headers : Vec<Field>
}
impl RequestHeader {
    pub fn new() -> RequestHeader {
        RequestHeader {
            headers : Vec::new()
        }
    }
    pub fn add(&mut self, field : Field) {
        self.headers.push(field);
    }
     pub fn required(&self) -> bool {
        self.headers.len()> 0
    }
}
#[derive(Serialize, Debug)]
struct ResponseBody{
    params : Vec<Field>
}
impl ResponseBody {
    pub fn new() -> ResponseBody {
        ResponseBody {
            params : Vec::new()
        }
    }
    pub fn add(&mut self, field : Field) {
        self.params.push(field);
    }
    pub fn required(&self) -> bool {
        self.params.len()> 0
    }
}
#[derive(Serialize, Debug)]
struct Field{
    field : String,
    param_type : String,
    required : bool,
    description: String,
    example : String,
    pure_type : String
}
impl Field {
    pub fn new(field : String, param_type : String, required : bool, description: String, example : String) -> Field {
        let pure_type = param_type.replace("[", "").replace("]","").replace("Map<string,", "").replace(">", "");
        Field {
            field,
            param_type ,
            required ,
            description,
            example,
            pure_type
        }
    }
}
#[derive(Serialize, Debug)]
struct QueryParameter {
    params : Vec<Field>
}
impl QueryParameter {
    pub fn new() -> QueryParameter {
        QueryParameter {
            params : Vec::new()
        }
    }
    pub fn add(&mut self, field : Field) {
        self.params.push(field);
    }
     pub fn required(&self) -> bool {
        self.params.len()> 0
    }
}
#[derive(Serialize, Debug)]
struct Schema {
    name : String,
    fields : Vec<Field>,
    enumFields : Vec<Field>,
    is_enum : bool
}
impl Schema {
    pub fn new (name : String, fields : Vec<Field>, enumFields : Vec<Field>) -> Schema {
        let is_enum = enumFields.len() > 0;
        Schema {
            name,
            fields,
            enumFields,
            is_enum
        }
    }
}

#[derive(Clone)]
pub enum HttpStatus {
    Status(&'static str,&'static str)
}
pub fn get_status_string_from_code(status_code : String) ->  &'static HttpStatus{
    match status_code.as_str() {
        "200" => {
            &HttpOk
        },
        "400" => {
            &HttpBadRequest
        }, 
        "403" => {
            &HttpForbidden
        },
        _ => {
            &HttpInternalServerError   
        }
    }
}

static HttpOk : HttpStatus = HttpStatus::Status("200", "Success");
static HttpForbidden : HttpStatus = HttpStatus::Status("403", "Forbidden");
static HttpBadRequest : HttpStatus = HttpStatus::Status("400", "Bad Request");
static HttpInternalServerError : HttpStatus = HttpStatus::Status("500", "Internal Server Error");

#[derive(Serialize, Debug)]
struct Response {
    params : Vec<Field>,
    application_json : bool,
    content_type : String,
    description : String,
    status_code : String,
    status_string : String,
    error : bool
}
impl Response {
     pub fn new() -> Response {
         let HttpStatus::Status(status_code,status_string) = HttpOk;
        Response {
            status_code : status_code.to_string(),
            status_string: status_string.to_string(),
            params : Vec::new(),
            application_json : true,
            content_type : String::from("application/json"),
            description : String::from(""),
            error : false
        }
    }
    pub fn add(&mut self, field : Field) {
        self.params.push(field);
    }
     pub fn required(&self) -> bool {
        self.params.len()> 0
    }
    pub fn set_content_type(&mut self, content_type : String) {
        if content_type.contains("application/json") {
            self.application_json = true;
        } else {
            self.application_json = false;
        }
        self.content_type = content_type;
    }
    pub fn set_description(&mut self,desc: String) {
        self.description = desc;
    }
    pub fn set_status_code(&mut self, code : String) {
        let HttpStatus::Status(status_code, status_string) = get_status_string_from_code(code);
        self.status_code = String::from(*status_code);
        self.status_string = String::from(*status_string);
        match self.status_code.as_str() {
            "200" | "302" => {
                self.error = false;
            },
            _ => {
                self.error = true;
            }
        }
    }
}

//line matches
static LIST_MARKDOWN : &'static str = r" *[-] (?P<item>.*)";

//inner matches
static EMPH_STAR : &'static str = "\\*(?P<content>.*?)\\*";
static EMPH_UNDERLINE : &'static str = "_(?P<content>.*?)_";
static STRONG_STAR : &'static str = "\\*\\*(?P<content>.*?)\\*\\*";
static STRONG_UNDERLINE : &'static str = "__(?P<content>.*?)__";
static LINK : &'static str = "\\[(?P<displayText>.*)\\]\\((?P<link>https?://[A-z0-9.-_\\\\/]*)( \"(?P<hover>.*)\")?\\)";
pub fn markdown_to_latex(markdown : &str) -> String {
    let mut newString = String::new();
    let mut inList = false;
    //they can only be matched once in a line
    let line_level_regex = RegexSet::new(&vec![LIST_MARKDOWN]).unwrap();

    let list_regex = regex::Regex::new(LIST_MARKDOWN).unwrap();
    for line in markdown.lines() {
        //ideally only one line level match occurs so take the first match
        if line_level_regex.is_match(line) {
            let lineLevelMatches = line_level_regex.matches(line).into_iter().next();
            match lineLevelMatches {
                Some(0) => {
                    //we have a list 
                    if !inList {
                        inList = true;
                        newString.push_str("\\begin{itemize}\n");
                    }
                    let result = list_regex.replace_all(line, "\\item $item");
                    
                    newString.push_str(&inner_replace(&result));
                },
                _ => {}
            }
        } else {
            //normal line replace all modifier
            if inList {
                inList = false;
                newString.push_str("\\end{itemize}\n");
            }
            newString.push_str(&inner_replace(&line));
        }
        newString.push('\n');
    }
    //if the text ends with a list we must ensure that we close the itemize environment
    if inList {
        inList = false;
        newString.push_str(r"\end{itemize}");
    }
    newString
}


fn inner_replace(string : &str) -> String {
    let modifier_regex = RegexSet::new(&vec![STRONG_STAR,STRONG_UNDERLINE,EMPH_STAR, EMPH_UNDERLINE,LINK]).unwrap();

    let all_matches = vec![regex::Regex::new(STRONG_STAR).unwrap(),regex::Regex::new(STRONG_UNDERLINE).unwrap(),regex::Regex::new(EMPH_STAR).unwrap(),regex::Regex::new(EMPH_UNDERLINE).unwrap(),regex::Regex::new(LINK).unwrap()];
    let replacements = vec![r"\textbf{$content}",r"\textbf{$content}", r"\emph{$content}",r"\emph{$content}",r"\url[$displayText]{$link}"];
    if !modifier_regex.is_match(string) {
       String::from(string)
    }
    else {
        let mut new_string = String::from(string);
        while modifier_regex.is_match(&new_string) {
            let first_match = modifier_regex.matches(&new_string).into_iter().next().unwrap();
            let result = all_matches[first_match].replace_all(&new_string, replacements[first_match]);
            new_string = String::from(result);
        }
        inner_replace(new_string.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_markdown_list () {
        let string = "- test\n- item2\n- a b c";
        assert_eq!(markdown_to_latex(string), "\\begin{itemize}\n\\item test\n\\item item2\n\\item a b c\n\\end{itemize}");
    }
    #[test]
    pub fn test_markdown_modifiers() {
        let emph = "*emph*";
        assert_eq!(markdown_to_latex(emph).trim(),"\\emph{emph}");
        let emph = "_emph_";
        assert_eq!(markdown_to_latex(emph).trim(),"\\emph{emph}");

        let strong = "**strong**";
        assert_eq!(markdown_to_latex(strong).trim(),"\\textbf{strong}");
        let strong = "__strong__";
        assert_eq!(markdown_to_latex(strong).trim(),"\\textbf{strong}");
         let strong = "***strongemph***";
        assert_eq!(markdown_to_latex(strong).trim(),"\\textbf{\\emph{strongemph}}");
        let strong = "___strongemph___";
        assert_eq!(markdown_to_latex(strong).trim(),"\\textbf{\\emph{strongemph}}");
          let strong = "**_strongemph_**";
        assert_eq!(markdown_to_latex(strong).trim(),"\\textbf{\\emph{strongemph}}");
        let strong = "__*strongemph*__";
        assert_eq!(markdown_to_latex(strong).trim(),"\\textbf{\\emph{strongemph}}");

        let url = "[test](https://www.google.ch \"Hallo Velo\")";
        assert_eq!(markdown_to_latex(url).trim(), "\\url[test]{https://www.google.ch}");
          let url = "[test](https://www.google.ch)";
        assert_eq!(markdown_to_latex(url).trim(), "\\url[test]{https://www.google.ch}");
    }

    #[test]
    pub fn test_line_and_inline() {
        let all_in_one = "**strong**\n - test1\n- *emph1*\n- [test](https://www.google.ch)\n_emph_";
        assert_eq!(markdown_to_latex(all_in_one).trim(), "\\textbf{strong}\n\\begin{itemize}\n\\item test1\n\\item \\emph{emph1}\n\\item \\url[test]{https://www.google.ch}\n\\end{itemize}\n\\emph{emph}");
        let all_in_one = "**strong**\n -test1\n- *emph1*\n- [test](https://www.google.ch)\n_emph_";
        assert_ne!(markdown_to_latex(all_in_one).trim(), "\\textbf{strong}\n\\begin{itemize}\n\\item test1\n\\item \\emph{emph1}\n\\item \\url[test]{https://www.google.ch}\n\\end{itemize}\n\\emph{emph}");

    }
    #[test]
    pub fn no_markdown() {
        let test = "hallo velo. normaler test\n";
        assert_eq!(markdown_to_latex(test), test);
    }

    #[test]
    pub fn lazy_match() {
        let test = "*next* **n**";
        assert_eq!(markdown_to_latex(test).trim(), "\\emph{next} \\textbf{n}");
    }
}
