pub struct Error {
    file_name: String, 
    line_number: usize,
    start: usize, 
    end: usize, 
    message: String
}