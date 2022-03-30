use core::fmt;
use std::{cmp::Ordering, fs, io, ops::Rem, path::PathBuf};

use console::{Key, Term};
use dialoguer::theme::{SimpleTheme, Theme};
use paging_copy::Paging;

mod paging_copy;

#[derive(Debug, Clone)]
pub enum FileType {
    Folder,
    WithExtension(String),
    Any,
}

impl Default for FileType {
    fn default() -> Self {
        FileType::Any
    }
}

pub struct FilePicker<'a> {
    file_type: FileType,
    // items: Vec<String>,
    prompt: Option<String>,
    report: bool,
    clear: bool,
    theme: &'a dyn Theme,
    max_length: Option<usize>,
    initial_folder: Option<PathBuf>,
}

impl Default for FilePicker<'static> {
    fn default() -> Self {
        Self::new(FileType::Any)
    }
}

impl FilePicker<'static> {
    /// Creates a select prompt builder with default theme.
    pub fn new(file_type: FileType) -> Self {
        Self::with_theme(file_type, &SimpleTheme)
    }
}

impl FilePicker<'_> {
    /// Indicates whether select menu should be erased from the screen after interaction.
    ///
    /// The default is to clear the menu.
    pub fn clear(&mut self, val: bool) -> &mut Self {
        self.clear = val;
        self
    }

    /// Sets an optional max length for a page.
    ///
    /// Max length is disabled by None
    pub fn max_length(&mut self, val: usize) -> &mut Self {
        // Paging subtracts two from the capacity, paging does this to
        // make an offset for the page indicator. So to make sure that
        // we can show the intended amount of items we need to add two
        // to our value.
        self.max_length = Some(val + 2);
        self
    }

    /// Sets the select prompt.
    ///
    /// By default, when a prompt is set the system also prints out a confirmation after
    /// the selection. You can opt-out of this with [`report`](#method.report).
    ///
    /// ## Examples
    /// ```rust,no_run
    /// use dialoguer::Select;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let selection = Select::new()
    ///         .with_prompt("Which option do you prefer?")
    ///         .item("Option A")
    ///         .item("Option B")
    ///         .interact()?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn with_prompt<S: Into<String>>(&mut self, prompt: S) -> &mut Self {
        self.prompt = Some(prompt.into());
        self.report = true;
        self
    }

    /// Indicates whether to report the selected value after interaction.
    ///
    /// The default is to report the selection.
    pub fn report(&mut self, val: bool) -> &mut Self {
        self.report = val;
        self
    }

    /// Enables user interaction and returns the result.
    ///
    /// The user can select the items with the 'Space' bar or 'Enter' and the index of selected item will be returned.
    /// The dialog is rendered on stderr.
    /// Result contains `index` if user selected one of items using 'Enter'.
    /// This unlike [`interact_opt`](Self::interact_opt) does not allow to quit with 'Esc' or 'q'.
    #[inline]
    pub fn interact(&self) -> io::Result<PathBuf> {
        self.interact_on(&Term::stderr())
    }

    /// Enables user interaction and returns the result.
    ///
    /// The user can select the items with the 'Space' bar or 'Enter' and the index of selected item will be returned.
    /// The dialog is rendered on stderr.
    /// Result contains `Some(index)` if user selected one of items using 'Enter' or `None` if user cancelled with 'Esc' or 'q'.
    #[inline]
    pub fn interact_opt(&self) -> io::Result<Option<PathBuf>> {
        self.interact_on_opt(&Term::stderr())
    }

    /// Like [interact](#method.interact) but allows a specific terminal to be set.
    ///
    /// ## Examples
    ///```rust,no_run
    /// use dialoguer::Select;
    /// use console::Term;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let selection = Select::new()
    ///         .item("Option A")
    ///         .item("Option B")
    ///         .interact_on(&Term::stderr())?;
    ///
    ///     println!("User selected option at index {}", selection);
    ///
    ///     Ok(())
    /// }
    ///```
    #[inline]
    pub fn interact_on(&self, term: &Term) -> io::Result<PathBuf> {
        self._interact_on(term, false)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Quit not allowed in this case"))
    }

    /// Like [`interact_opt`](Self::interact_opt) but allows a specific terminal to be set.
    ///
    /// ## Examples
    /// ```rust,no_run
    /// use dialoguer::Select;
    /// use console::Term;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let selection = Select::new()
    ///         .item("Option A")
    ///         .item("Option B")
    ///         .interact_on_opt(&Term::stdout())?;
    ///
    ///     match selection {
    ///         Some(position) => println!("User selected option at index {}", position),
    ///         None => println!("User did not select anything or exited using Esc or q")
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    #[inline]
    pub fn interact_on_opt(&self, term: &Term) -> io::Result<Option<PathBuf>> {
        self._interact_on(term, true)
    }

    /// Like `interact` but allows a specific terminal to be set.
    fn _interact_on(&self, term: &Term, allow_quit: bool) -> io::Result<Option<PathBuf>> {
        let mut directory = match &self.initial_folder {
            Some(folder) => folder.clone(),
            None => std::env::current_dir()?,
        };

        'directory: loop {
            let files_in_dir = FilePicker::list_files_in_folder(&directory, &self.file_type)?;
            let filenames: Vec<String> = files_in_dir
                .iter()
                .map(|path| {
                    path.file_name()
                        .expect("Filename existance checked in list function")
                        .to_string_lossy()
                        .into()
                })
                .collect();

            let mut paging = Paging::new(term, filenames.len(), self.max_length);
            let mut render = TermThemeRenderer::new(term, self.theme);
            let mut sel = 0;

            let mut size_vec = Vec::new();

            for items in filenames
                .iter()
                .flat_map(|i| i.split('\n'))
                .collect::<Vec<_>>()
            {
                let size = &items.len();
                size_vec.push(*size);
            }

            term.hide_cursor()?;

            loop {
                if let Some(ref prompt) = self.prompt {
                    paging
                        .render_prompt(|paging_info| render.select_prompt(prompt, paging_info))?;
                }

                for (idx, item) in filenames
                    .iter()
                    .enumerate()
                    .skip(paging.current_page * paging.capacity)
                    .take(paging.capacity)
                {
                    render.select_prompt_item(item, sel == idx)?;
                }

                term.flush()?;

                match term.read_key()? {
                    Key::ArrowDown | Key::Tab | Key::Char('j') => {
                        if sel == !0 {
                            sel = 0;
                        } else {
                            sel = (sel as u64 + 1).rem(filenames.len() as u64) as usize;
                        }
                    }
                    Key::Escape | Key::Char('q') => {
                        if allow_quit {
                            if self.clear {
                                render.clear()?;
                            } else {
                                term.clear_last_lines(paging.capacity)?;
                            }

                            term.show_cursor()?;
                            term.flush()?;

                            return Ok(None);
                        }
                    }
                    Key::ArrowUp | Key::BackTab | Key::Char('k') => {
                        if sel == !0 {
                            sel = filenames.len() - 1;
                        } else {
                            sel = ((sel as i64 - 1 + filenames.len() as i64)
                                % (filenames.len() as i64))
                                as usize;
                        }
                    }
                    Key::ArrowLeft | Key::Char('h') => {
                        if paging.active {
                            sel = paging.previous_page();
                        }
                    }
                    Key::ArrowRight | Key::Char('l') => {
                        if paging.active {
                            sel = paging.next_page();
                        }
                    }

                    Key::Enter if sel != !0 => {
                        if self.clear {
                            render.clear()?;
                        }

                        if let Some(ref prompt) = self.prompt {
                            if self.report {
                                render.select_prompt_selection(prompt, &filenames[sel])?;
                            }
                        }

                        term.show_cursor()?;
                        term.flush()?;

                        return Ok(Some(files_in_dir[sel].clone()));
                    }
                    Key::Char(' ') if sel != !0 => {
                        if self.clear {
                            render.clear()?;
                        }

                        if let Some(ref prompt) = self.prompt {
                            if self.report {
                                render.select_prompt_selection(prompt, &filenames[sel])?;
                            }
                        }
                        let current = &files_in_dir[sel];
                        if current.is_dir() {
                            render.clear()?;
                            directory = current.clone();
                            continue 'directory;
                        } else {
                            term.show_cursor()?;
                            term.flush()?;

                            return Ok(Some(files_in_dir[sel].clone()));
                        }
                    }
                    _ => {}
                }

                paging.update(sel)?;

                if paging.active {
                    render.clear()?;
                } else {
                    render.clear_preserve_prompt(&size_vec)?;
                }
            }
        }
    }

    fn list_files_in_folder(folder: &PathBuf, file_type: &FileType) -> io::Result<Vec<PathBuf>> {
        fn entry_match(entry: &PathBuf, file_type: &FileType) -> bool {
            if entry.file_name().is_none() {
                return false;
            }

            match file_type {
                FileType::Folder => entry.is_dir(),
                FileType::WithExtension(extension) => {
                    entry.is_dir()
                        || entry
                            .extension()
                            .filter(|os_ext| {
                                extension.cmp(&os_ext.to_string_lossy().to_lowercase())
                                    == Ordering::Equal
                            })
                            .is_some()
                }
                FileType::Any => true,
            }
        }

        let content: Vec<_> = fs::read_dir(folder)?
            .filter_map(|content| content.ok().map(|entry| entry.path()))
            .filter(|entry| entry_match(entry, file_type))
            .collect();

        Ok(content)
    }
}

impl<'a> FilePicker<'a> {
    /// Creates a select prompt builder with a specific theme.
    ///
    /// ## Examples
    /// ```rust,no_run
    /// use dialoguer::{
    ///     Select,
    ///     theme::ColorfulTheme
    /// };
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let selection = Select::with_theme(&ColorfulTheme::default())
    ///         .item("Option A")
    ///         .item("Option B")
    ///         .interact()?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn with_theme(file_type: FileType, theme: &'a dyn Theme) -> Self {
        Self {
            file_type,
            prompt: None,
            report: false,
            clear: true,
            max_length: None,
            theme,
            initial_folder: None,
        }
    }
}

pub(crate) struct TermThemeRenderer<'a> {
    term: &'a Term,
    theme: &'a dyn Theme,
    height: usize,
    prompt_height: usize,
    prompts_reset_height: bool,
}

impl<'a> TermThemeRenderer<'a> {
    pub fn new(term: &'a Term, theme: &'a dyn Theme) -> TermThemeRenderer<'a> {
        TermThemeRenderer {
            term,
            theme,
            height: 0,
            prompt_height: 0,
            prompts_reset_height: true,
        }
    }

    fn write_formatted_line<
        F: FnOnce(&mut TermThemeRenderer, &mut dyn fmt::Write) -> fmt::Result,
    >(
        &mut self,
        f: F,
    ) -> io::Result<()> {
        let mut buf = String::new();
        f(self, &mut buf).map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        self.height += buf.chars().filter(|&x| x == '\n').count() + 1;
        self.term.write_line(&buf)
    }

    fn write_formatted_prompt<
        F: FnOnce(&mut TermThemeRenderer, &mut dyn fmt::Write) -> fmt::Result,
    >(
        &mut self,
        f: F,
    ) -> io::Result<()> {
        self.write_formatted_line(f)?;
        if self.prompts_reset_height {
            self.prompt_height = self.height;
            self.height = 0;
        }
        Ok(())
    }

    fn write_paging_info(buf: &mut dyn fmt::Write, paging_info: (usize, usize)) -> fmt::Result {
        write!(buf, " [Page {}/{}] ", paging_info.0, paging_info.1)
    }

    pub fn select_prompt(
        &mut self,
        prompt: &str,
        paging_info: Option<(usize, usize)>,
    ) -> io::Result<()> {
        self.write_formatted_prompt(|this, buf| {
            this.theme.format_select_prompt(buf, prompt)?;

            if let Some(paging_info) = paging_info {
                TermThemeRenderer::write_paging_info(buf, paging_info)?;
            }

            Ok(())
        })
    }

    pub fn select_prompt_selection(&mut self, prompt: &str, sel: &str) -> io::Result<()> {
        self.write_formatted_prompt(|this, buf| {
            this.theme.format_select_prompt_selection(buf, prompt, sel)
        })
    }

    pub fn select_prompt_item(&mut self, text: &str, active: bool) -> io::Result<()> {
        self.write_formatted_line(|this, buf| {
            this.theme.format_select_prompt_item(buf, text, active)
        })
    }
    pub fn clear(&mut self) -> io::Result<()> {
        self.term
            .clear_last_lines(self.height + self.prompt_height)?;
        self.height = 0;
        Ok(())
    }

    pub fn clear_preserve_prompt(&mut self, size_vec: &[usize]) -> io::Result<()> {
        let mut new_height = self.height;
        //Check each item size, increment on finding an overflow
        for size in size_vec {
            if *size > self.term.size().1 as usize {
                new_height += 1;
            }
        }

        self.term.clear_last_lines(new_height)?;
        self.height = 0;
        Ok(())
    }
}
