// This file is part of oraide.  See <https://github.com/Phrohdoh/oraide>.
// 
// oraide is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License version 3
// as published by the Free Software Foundation.
// 
// oraide is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// 
// You should have received a copy of the GNU Affero General Public License
// along with oraide.  If not, see <https://www.gnu.org/licenses/>.

use std::{
    path::{
        PathBuf,
    },
};

use oraide_span::FileId;
use oraide_parser_miniyaml::{
    FilesCtx as _,
    TextFilesCtx as _,
    ParserCtx as _,
};
use oraide_query_system::OraideDatabase;

pub(crate) struct FindDefinition {
    name_to_find: String,
    file_ids: Vec<FileId>,
    db: OraideDatabase,
}

impl FindDefinition {
    pub(crate) fn new(name_to_find: String, project_root_dir: PathBuf) -> Result<Self, String> {
        let mut db = OraideDatabase::default();

        let dir_walker = walkdir::WalkDir::new(&project_root_dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.metadata().map(|md| md.is_file()).unwrap_or(false))
            .filter(|entry| entry.path().extension() == Some(std::ffi::OsString::from("yaml".to_string()).as_ref()))
            ;

        let file_ids = dir_walker
            .map(|file| crate::add_file(&mut db, file.path()))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            name_to_find,
            file_ids,
            db,
        })
    }

    pub(crate) fn run(&self) {
        for file_id in self.file_ids.iter() {
            if let Some(node) = self.db.top_level_node_by_key_in_file(*file_id, self.name_to_find.clone()) {
                let span = node.span().unwrap();
                let file_name = self.db.file_path(*file_id).unwrap();
                let start = span.start();
                let loc = self.db.convert_byte_index_to_location(*file_id, start).unwrap();
                println!("{}:{}", file_name, loc);
            }
        }
    }
}