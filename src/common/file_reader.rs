pub trait FileReader {
    fn buffer_size(&self) -> u32;
    fn seek(&mut self, max_size: u32) -> Result<Option<&[u8]>, Box<dyn std::error::Error>>;
}
