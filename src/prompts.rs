pub struct Prompt {  
}

impl Prompt {
    pub fn get_prompt(prompt: &str) -> &str {
        return match prompt {
            _ => "",
        }
    }
}
