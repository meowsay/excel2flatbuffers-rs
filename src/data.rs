use calamine::{open_workbook, Reader, Xlsx};
use std::fmt;
use std::fs;
use std::path::Path;
extern crate flatbuffers;
use flatbuffers::{TableFinishedWIPOffset, WIPOffset};
use std::io::{BufWriter, Write};

#[derive(Debug)]
enum DataValue<'fbb> {
    FString(WIPOffset<&'fbb str>),
    Int(i32),
    Float(f32),
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum TableDataType {
    int,
    string,
    float,
    long,
}

impl fmt::Display for TableDataType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub struct Header {
    pub cell_index: usize,
    pub name: String,
    pub data_type: TableDataType,
    pub is_comment: bool,
}

impl Header {
    pub fn new(
        cell_index: usize,
        name: String,
        data_type: TableDataType,
        is_comment: bool,
    ) -> Self {
        Header {
            cell_index,
            name,
            data_type,
            is_comment,
        }
    }

    pub fn new_normal(cell_index: usize, name: String, data_type: TableDataType) -> Self {
        Header {
            cell_index,
            name,
            data_type,
            is_comment: false,
        }
    }

    pub fn new_comment(cell_index: usize) -> Self {
        Header {
            cell_index,
            name: String::new(),
            data_type: TableDataType::int,
            is_comment: true,
        }
    }
}

#[derive(Debug)]
pub struct RawSheet {
    pub sheet_name: String, // sheet name
    pub data: Vec<Vec<String>>,
    pub header: Vec<Header>,
    pub namespace: String,
}

impl RawSheet {
    pub fn new(sheet_name: String, data: Vec<Vec<String>>, namespace: &str) -> Self {
        if let Some(header_row) = data.get(0) {
            let header = RawSheet::parse_header(header_row);
            Self {
                sheet_name,
                data,
                header,
                namespace: String::from(namespace),
            }
        } else {
            panic!("解析Header失败: {}", sheet_name);
        }
    }

    pub fn pack_data(
        &self,
        output_dir: &str,
        file_identifier: Option<&str>,
    ) -> Result<(), std::io::Error> {
        let file_path = format!("{}{}.bytes", output_dir, self.sheet_name);
        //println!("Handle Sheet: {:?}", self.sheet_name);
        let mut row_data_vec: Vec<WIPOffset<TableFinishedWIPOffset>> = Vec::new();
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        for i in 1..self.data.len() {
            if let Some(row) = self.data.get(i) {
                let row_data = self.pack_row(&mut builder, row);
                row_data_vec.push(row_data);
            }
        }

        let data = builder.create_vector(&row_data_vec[..]);
        let start = builder.start_table();
        builder.push_slot_always(4, data);
        let o = builder.end_table(start);
        builder.finish(o, file_identifier);
        let buf = builder.finished_data();

        let mut writer: Box<dyn std::io::Write> =
            Box::new(BufWriter::new(fs::File::create(file_path)?));
        writer.write_all(&buf)?;

        Ok(())
    }

    fn pack_row(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder,
        row: &[String],
    ) -> WIPOffset<TableFinishedWIPOffset> {
        let mut value_vec: Vec<DataValue> = Vec::new();

        // 解析一行中的值
        for i in 0..row.len() {
            if let Some(header_field) = self.header.get(i) {
                if !header_field.is_comment {
                    if let Some(svalue) = row.get(i) {
                        let data_value = match header_field.data_type {
                            TableDataType::int => {
                                DataValue::Int(svalue.parse::<i32>().unwrap_or(0))
                            }
                            TableDataType::float => {
                                DataValue::Float(svalue.parse::<f32>().unwrap_or(0.0))
                            }
                            TableDataType::long => {
                                DataValue::Int(svalue.parse::<i32>().unwrap_or(0))
                            }
                            TableDataType::string => {
                                DataValue::FString(builder.create_string(svalue))
                            }
                        };
                        value_vec.push(data_value);
                    }
                }
            }
        }

        let start = builder.start_table();
        let mut offset = 4;
        for dvalue in value_vec.iter() {
            match dvalue {
                DataValue::Int(value) => {
                    builder.push_slot::<i32>(offset, *value, 0);
                }
                DataValue::Float(value) => {
                    builder.push_slot::<f32>(offset, *value, 0.0);
                }
                DataValue::FString(value) => {
                    builder.push_slot_always::<flatbuffers::WIPOffset<_>>(offset, *value);
                }
            }
            offset += 2;
        }

        builder.end_table(start)
    }

    fn parse_header(header_row: &[String]) -> Vec<Header> {
        let mut header_vec: Vec<Header> = Vec::new();
        let mut cell_index: usize = 0;
        for field in header_row {
            if field.starts_with('#') {
                header_vec.push(Header::new_comment(cell_index));
            } else {
                let field_split_vec: Vec<&str> = field
                    .split('|')
                    .filter(|word| !word.trim().is_empty())
                    .collect();
                if field_split_vec.len() == 2 {
                    let real_data = field_split_vec[1];
                    let data_vec: Vec<&str> = real_data.split('(').collect();
                    if data_vec.len() >= 2 {
                        let field_name = String::from(data_vec[0]);
                        let data_type_part = data_vec[1].trim().to_lowercase();
                        let data_type = if data_type_part.contains("int32") {
                            TableDataType::int
                        } else if data_type_part.contains("string") {
                            TableDataType::string
                        } else if data_type_part.contains("float") {
                            TableDataType::float
                        } else {
                            panic!("Unknow data type: {}", data_type_part);
                        };
                        header_vec.push(Header::new_normal(cell_index, field_name, data_type));
                    } else {
                        panic!("field error: {}", real_data);
                    }
                } else {
                    header_vec.push(Header::new_comment(cell_index));
                    // panic!("field error: {}", field);
                }
            }
            cell_index += 1;
        }

        header_vec
    }

    fn generate_logic_lua_code(&self, table_root: &str) -> String {
        let mut define_code_lines: Vec<String> = Vec::new();
        for header in self.header.iter() {
            if !header.is_comment {
                let code = format!("        define.{0} = data:{0}()", header.name);
                define_code_lines.push(code);
            }
        }

        let define_code_str = define_code_lines.join("\n");

        let table_code_str = format!(
            "
local {0} = require \"{3}.AutoGenConfig.{0}\"

local {0}TableClass = BaseClass(\"{0}TableClass\", BaseConfigTableClass)

function {0}TableClass:OnLoad(buff)
    local config = {0}.GetRootAs{0}(buff, 0)
    for i = 1, config:DataLength() do
        local data = config:Data(i)
        local define = {2}
{1}
    
        self:AddToDict(define.ID, define)
    end
end

return {0}TableClass
        ",
            self.sheet_name, define_code_str, "{}", table_root
        );

        table_code_str
    }

    pub fn generate_fbs_code(&self) -> String {
        let table_code_str = format!(
            "
table {} {{
    data: [Single{}Data];
}}

root_type {};
            ",
            self.sheet_name, self.sheet_name, self.sheet_name
        );

        let mut variables_code = String::new();
        for header in self.header.iter() {
            if !header.is_comment {
                let code = format!(
                    "    {}:{};\n",
                    header.name,
                    header.data_type.to_string().to_lowercase()
                );
                variables_code.push_str(&code);
            }
        }

        let single_table_code_str = format!(
            "
table Single{}Data {{
{}
}}
            ",
            self.sheet_name, variables_code
        );

        // println!("Namespace: {}", self.namespace);

        let single_table_code_str = if self.namespace != "" {
            format!("namespace {};\n{}", self.namespace, single_table_code_str)
        } else {
            single_table_code_str
        };

        // println!("single_table_code: \n{}", single_table_code_str);

        let mut fbs_code = String::new();
        fbs_code.push_str(&single_table_code_str);
        fbs_code.push_str(&table_code_str);

        fbs_code
    }

    pub fn write_to_fbs_file(&self, output_dir: &str) -> Result<(), std::io::Error> {
        let output_file = format!("{}{}.fbs", output_dir, self.sheet_name);
        if !Path::new(output_dir).is_dir() {
            fs::create_dir(output_dir)?;
        }

        let fbs_code = self.generate_fbs_code();

        fs::write(output_file, &fbs_code)?;
        println!("生成：{}", self.sheet_name);

        Ok(())
    }

    pub fn write_to_logic_lua_file(
        &self,
        output_dir: &str,
        table_root: &str,
    ) -> Result<(), std::io::Error> {
        let output_file = format!("{}{}TableClass.lua", output_dir, self.sheet_name);
        if !Path::new(output_dir).is_dir() {
            fs::create_dir(output_dir)?;
        }
        let code = self.generate_logic_lua_code(table_root);
        fs::write(output_file, &code)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct RawTable {
    pub excel_path: String,
    pub sheets: Vec<RawSheet>,
}

impl RawTable {
    pub fn new(excel_path: &str, namespace: &str) -> Option<Self> {
        if let Some(sheet_vec) = RawTable::read_excel(excel_path, namespace) {
            let path = String::from(excel_path);
            Some(Self {
                excel_path: path,
                sheets: sheet_vec,
            })
        } else {
            None
        }
    }

    fn read_excel(excel_path: &str, namespace: &str) -> Option<Vec<RawSheet>> {
        if let Ok(mut workbook) = open_workbook::<Xlsx<_>, &str>(excel_path) {
            let sheets = workbook.sheet_names().to_owned();

            let mut sheet_vec: Vec<RawSheet> = Vec::new();

            for sheet in sheets {
                if sheet.contains("【") {
                    continue;
                }
                if let Some(Ok(range)) = workbook.worksheet_range(&sheet) {
                    let mut row_vec = Vec::new();
                    for row in range.rows() {
                        let vec: Vec<String> = row.iter().map(|a| a.to_string()).collect();
                        row_vec.push(vec);
                    }
                    // let excel_name = String::from(excel_path);
                    sheet_vec.push(RawSheet::new(sheet, row_vec, namespace));
                } else {
                    eprintln!("Error with sheet: {0}", sheet);
                }
            }

            Some(sheet_vec)
        } else {
            None
        }
    }

    pub fn write_to_fbs_file(&self, output_dir: &str) -> Result<(), std::io::Error> {
        for sheet in self.sheets.iter() {
            sheet.write_to_fbs_file(output_dir)?;
        }

        Ok(())
    }

    pub fn write_to_logic_lua_file(
        &self,
        output_dir: &str,
        table_root: &str,
    ) -> Result<(), std::io::Error> {
        for sheet in self.sheets.iter() {
            sheet.write_to_logic_lua_file(output_dir, table_root)?;
        }

        Ok(())
    }

    //     pub fn write_to_logic_lua_mod_file(&self, output_dir: &str) -> Result<(), std::io::Error> {
    //         let mut line_vec: Vec<String> = Vec::new();

    //         for sheet in self.sheets.iter() {
    //             let sheet_name = sheet.sheet_name.clone();
    //             let code_str = format!(
    //                 "
    // local {0}TableClass = require \"Game.ConfigTables.{0}TableClass\"
    // {0}Table = {0}TableClass.New(\"ConfigBytes/{0}\")
    // ConfigTableST:GetInstance():AddTable({0}Table)

    //             ",
    //                 sheet_name
    //             );
    //             line_vec.push(code_str);
    //         }

    //         let code = line_vec.join("\n");

    //         let output_file = format!("{}Mod.lua", output_dir);
    //         if !Path::new(output_dir).is_dir() {
    //             fs::create_dir(output_dir)?;
    //         }
    //         fs::write(output_file, &code)?;

    //         Ok(())
    //     }

    pub fn pack_data(
        &self,
        output_dir: &str,
        file_identifier: Option<&str>,
    ) -> Result<(), std::io::Error> {
        //println!("Pack Data, ExcelPath:{:?}", self.excel_path);
        for sheet in self.sheets.iter() {
            sheet.pack_data(output_dir, file_identifier)?;
        }
        Ok(())
    }
}
