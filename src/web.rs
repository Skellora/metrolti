use iron::prelude::*;
use iron::status;
use iron::middleware::Handler;
use mount::Mount;
use staticfile::Static;

use handlebars_iron::*;

pub fn startup_web_frontend(address: String, websocket_address: String, static_path: String) {
    let data = WebData {
        websocket: websocket_address.clone(),
        test: "Test".to_string(),
    };
    let mut chain = Chain::new(H { data: data });

    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(DirectorySource::new("./www/", ".hbs")));

    // load templates from all registered sources
    if let Err(r) = hbse.reload() {
        panic!("{}", r);
    }

    chain.link_after(hbse);

    let mut mount = Mount::new();
    mount.mount("/", chain)
         .mount("static", Static::new(static_path));
  
    Iron::new(mount).http(address).unwrap();
}

#[derive(Debug, Serialize)]
struct WebData {
    websocket: String,
    test: String,
}

struct H {
    data: WebData,
}

impl Handler for H {
    fn handle(&self, _: &mut Request) -> IronResult<Response> {
        Ok(Response::with((status::Ok, Template::new("index", &self.data))))
    }
}
