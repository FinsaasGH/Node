// Copyright (c) 2019-2021, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use crate::line_reader::{TerminalEvent, TerminalReal};
use linefeed::{Interface, ReadResult, Writer};
use masq_lib::constants::MASQ_PROMPT;
use masq_lib::intentionally_blank;
use std::sync::Arc;

#[cfg(test)]
use linefeed::memory::MemoryTerminal;

#[cfg(not(test))]
use linefeed::DefaultTerminal;

//this is a layer with the broadest functionality, an object which is intended for you to usually work with at other
//places in the code

pub struct TerminalWrapper {
    interface: Arc<Box<dyn Terminal + Send + Sync>>,
}

impl TerminalWrapper {
    pub fn lock(&mut self) -> Box<dyn WriterGeneric + '_> {
        self.interface.provide_lock()
    }

    pub fn read_line(&self) -> TerminalEvent {
        self.interface.read_line()
    }

    pub fn add_history_unique(&self, line: String) {
        self.interface.add_history_unique(line)
    }

    pub fn new(interface: Box<dyn Terminal + Send + Sync>) -> Self {
        Self {
            interface: Arc::new(interface),
        }
    }

    #[cfg(test)]
    pub fn configure_interface() -> Result<Self, String> {
        Self::configure_interface_generic(Box::new(result_wrapper_for_in_memory_terminal))
    }

    #[cfg(not(test))]
    pub fn configure_interface() -> Result<Self, String> {
        //tested only for a negative result (an integration test)
        //no positive automatic test for this; tested by the fact that masq in interactive mode is runnable and passes human tests
        Self::configure_interface_generic(Box::new(DefaultTerminal::new))
    }

    fn configure_interface_generic<F, U>(terminal_creator_by_type: Box<F>) -> Result<Self, String>
    where
        F: FnOnce() -> std::io::Result<U>,
        U: linefeed::Terminal + 'static,
    {
        let interface =
            configure_interface(Box::new(Interface::with_term), terminal_creator_by_type)?;
        Ok(Self::new(Box::new(interface)))
    }

    #[cfg(test)]
    pub fn test_interface(&self) -> MemoryTerminal {
        self.interface.test_interface()
    }
}

impl Clone for TerminalWrapper {
    fn clone(&self) -> Self {
        Self {
            interface: Arc::clone(&self.interface),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unnecessary_wraps)]
fn result_wrapper_for_in_memory_terminal() -> std::io::Result<MemoryTerminal> {
    Ok(MemoryTerminal::new())
}

fn configure_interface<F, U, E: ?Sized, D>(
    interface_raw: Box<F>,
    terminal_type: Box<E>,
) -> Result<TerminalReal, String>
where
    F: FnOnce(&'static str, U) -> std::io::Result<D>,
    E: FnOnce() -> std::io::Result<U>,
    U: linefeed::Terminal + 'static,
    D: InterfaceRaw + Send + Sync + 'static,
{
    let terminal: U = match terminal_type() {
        Ok(term) => term,
        Err(e) => return Err(format!("Local terminal recognition: {}", e)),
    };
    let mut interface: Box<dyn InterfaceRaw + Send + Sync + 'static> =
        match interface_raw("masq", terminal) {
            Ok(interface) => Box::new(interface),
            Err(e) => return Err(format!("Preparing terminal interface: {}", e)),
        };

    if let Err(e) = set_all_settable_or_give_an_error(&mut *interface) {
        return Err(e);
    };

    Ok(TerminalReal::new(interface))
}

////////////////////////////////////////////////////////////////////////////////////////////////////

fn set_all_settable_or_give_an_error<U>(interface: &mut U) -> Result<(), String>
where
    U: InterfaceRaw + Send + Sync + 'static + ?Sized,
{
    if let Err(e) = interface.set_prompt(MASQ_PROMPT) {
        return Err(format!("Setting prompt: {}", e));
    }

    //here we can add some other parameter to be configured,
    //such as "completer" (see linefeed library)

    Ok(())
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait Terminal {
    fn provide_lock(&self) -> Box<dyn WriterGeneric + '_> {
        intentionally_blank!()
    }
    fn read_line(&self) -> TerminalEvent {
        intentionally_blank!()
    }
    fn add_history_unique(&self, _line: String) {}

    #[cfg(test)]
    fn test_interface(&self) -> MemoryTerminal {
        intentionally_blank!()
    }
    #[cfg(test)]
    fn tell_me_who_you_are(&self) -> String {
        intentionally_blank!()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

//declaration of TerminalReal is in line_reader.rs

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct TerminalInactive {}

impl Terminal for TerminalInactive {
    fn provide_lock(&self) -> Box<dyn WriterGeneric + '_> {
        Box::new(WriterInactive {})
    }
    #[cfg(test)]
    fn tell_me_who_you_are(&self) -> String {
        "TerminalIdle".to_string()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait WriterGeneric {
    fn write_str(&mut self, _str: &str) -> std::io::Result<()> {
        intentionally_blank!()
    }

    //I failed in attempts to use Any and dynamical casting from Box<dyn WriterGeneric>
    //because: Writer doesn't implement Clone and many if not all methods of Any require
    //'static, that is, it must be an owned object and I cannot get anything else but a referenced
    //Writer.
    //For delivering at least some test I decided to use this unusual hack
    #[cfg(test)]
    fn tell_me_who_you_are(&self) -> String {
        intentionally_blank!()
    }
}

impl<U: linefeed::Terminal> WriterGeneric for Writer<'_, '_, U> {
    fn write_str(&mut self, str: &str) -> std::io::Result<()> {
        self.write_str(&format!("{}\n*/-", str))
    }

    #[cfg(test)]
    fn tell_me_who_you_are(&self) -> String {
        "linefeed::Writer<_>".to_string()
    }
}

#[derive(Clone)]
pub struct WriterInactive {}

impl WriterGeneric for WriterInactive {
    #[cfg(test)]
    fn tell_me_who_you_are(&self) -> String {
        "WriterInactive".to_string()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait InterfaceRaw {
    fn read_line(&self) -> std::io::Result<ReadResult>;
    fn add_history_unique(&self, line: String);
    fn lock_writer_append(&self) -> std::io::Result<Box<dyn WriterGeneric + '_>>;
    fn set_prompt(&self, prompt: &str) -> std::io::Result<()>;
}

impl<U: linefeed::Terminal + 'static> InterfaceRaw for Interface<U> {
    fn read_line(&self) -> std::io::Result<ReadResult> {
        self.read_line()
    }

    fn add_history_unique(&self, line: String) {
        self.add_history_unique(line);
    }

    fn lock_writer_append(&self) -> std::io::Result<Box<dyn WriterGeneric + '_>> {
        match self.lock_writer_append() {
            Ok(writer) => Ok(Box::new(writer)),
            //untested ...dunno how to trigger any error here
            Err(error) => Err(error),
        }
    }

    fn set_prompt(&self, prompt: &str) -> std::io::Result<()> {
        self.set_prompt(prompt)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::mocks::{InterfaceRawMock, MixingStdout, TerminalActiveMock};
    use crate::test_utils::{written_output_all_lines, written_output_by_line_number};
    use crossbeam_channel::unbounded;
    use linefeed::DefaultTerminal;
    use std::io::{Error, Write};
    use std::sync::Barrier;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn terminal_mock_and_test_tools_write_and_read() {
        let mock = TerminalActiveMock::new()
            .read_line_result("Rocket, go to Mars, go, go".to_string())
            .read_line_result("And once again...nothing".to_string());

        let mut terminal = TerminalWrapper::new(Box::new(mock));
        let mut terminal_clone = terminal.clone();
        let terminal_reference = terminal.clone();

        terminal.lock().write_str("first attempt").unwrap();

        let handle = thread::spawn(move || {
            terminal_clone.lock().write_str("hello world").unwrap();
            terminal_clone.lock().write_str("that's enough").unwrap()
        });

        handle.join().unwrap();

        terminal.read_line();

        terminal.read_line();

        let lines_remaining = terminal_reference
            .test_interface()
            .lines()
            .lines_remaining();
        assert_eq!(lines_remaining, 24);

        let written_output =
            written_output_all_lines(terminal_reference.test_interface().lines(), true);
        assert_eq!(
            written_output,
            "first attempt | hello world | that's enough | \
         Rocket, go to Mars, go, go | And once again...nothing |"
        );

        let single_line =
            written_output_by_line_number(terminal_reference.test_interface().lines(), 1);
        assert_eq!(single_line, "first attempt");

        let single_line =
            written_output_by_line_number(terminal_reference.test_interface().lines(), 2);
        assert_eq!(single_line, "hello world")
    }

    //In the two following tests I use the system stdout handles, which is the standard way in the project, but thanks to
    //the lock from TerminalWrapper, it will be protected from one influencing another.

    #[test]
    fn terminal_wrapper_without_lock_does_not_block_others_from_writing_into_stdout() {
        let closure1: Box<dyn FnMut(TerminalWrapper, MixingStdout) + Sync + Send> =
            Box::new(move |_interface: TerminalWrapper, mut stdout_c| {
                write_in_cycles("AAA", &mut stdout_c);
            });

        let closure2: Box<dyn FnMut(TerminalWrapper, MixingStdout) + Sync + Send> =
            Box::new(move |_interface: TerminalWrapper, mut stdout_c| {
                write_in_cycles("BBB", &mut stdout_c);
            });

        let given_output = test_terminal_collision(Box::new(closure1), Box::new(closure2));

        //in an extreme case it may be printed as one is complete and the other sequence is interrupted
        assert!(
            !given_output.contains(&"A".repeat(90)) && !given_output.contains(&"B".repeat(90)),
            "without synchronization: {}",
            given_output
        );
    }

    #[test]
    fn terminal_wrapper_s_lock_blocks_others_to_write_into_stdout() {
        let closure1: Box<dyn FnMut(TerminalWrapper, MixingStdout) + Sync + Send> = Box::new(
            move |mut interface: TerminalWrapper, mut stdout_c: MixingStdout| {
                let _lock = interface.lock();
                write_in_cycles("AAA", &mut stdout_c);
            },
        );

        let closure2: Box<dyn FnMut(TerminalWrapper, MixingStdout) + Sync + Send> = Box::new(
            move |mut interface: TerminalWrapper, mut stdout_c: MixingStdout| {
                let _lock = interface.lock();
                write_in_cycles("BBB", &mut stdout_c);
            },
        );

        let given_output = test_terminal_collision(Box::new(closure1), Box::new(closure2));

        assert!(
            given_output.contains(&"A".repeat(90)),
            "synchronized: {}",
            given_output
        );
        assert!(
            given_output.contains(&"B".repeat(90)),
            "synchronized: {}",
            given_output
        );
    }

    fn test_terminal_collision<C>(closure1: Box<C>, closure2: Box<C>) -> String
    where
        C: FnMut(TerminalWrapper, MixingStdout) -> () + Sync + Send + 'static,
    {
        let interface = TerminalWrapper::new(Box::new(TerminalActiveMock::new()));

        let barrier = Arc::new(Barrier::new(2));

        let (tx, rx) = unbounded();
        let stdout_c1 = MixingStdout::new(tx);
        let stdout_c2 = stdout_c1.clone();

        let handles: Vec<_> = vec![(closure1, stdout_c1), (closure2, stdout_c2)]
            .into_iter()
            .map(|pair| {
                let (mut closure, stdout): (Box<C>, MixingStdout) = pair;
                let barrier_handle = Arc::clone(&barrier);
                let thread_interface = interface.clone();

                thread::spawn(move || {
                    barrier_handle.wait();
                    closure(thread_interface, stdout)
                })
            })
            .collect();

        handles
            .into_iter()
            .for_each(|handle| handle.join().unwrap());

        let mut buffer = String::new();
        loop {
            match rx.try_recv() {
                Ok(string) => buffer.push_str(&string),
                Err(_) => break buffer,
            }
        }
    }

    fn write_in_cycles(written_signal: &str, stdout: &mut dyn Write) {
        (0..30).for_each(|_| {
            write!(stdout, "{}", written_signal).unwrap();
            thread::sleep(Duration::from_millis(1))
        })
    }

    #[test]
    fn configure_interface_complains_that_there_is_no_real_terminal() {
        let subject = configure_interface(
            Box::new(Interface::with_term),
            Box::new(DefaultTerminal::new),
        );

        let result = match subject {
            Ok(_) => panic!("should have been an error, got OK"),
            Err(e) => e,
        };

        assert!(result.contains("Local terminal recognition:"), "{}", result);
        //Windows: The handle is invalid. (os error 6)
        //Linux: "Getting terminal parameters: Inappropriate ioctl for device (os error 25)"
    }

    #[test]
    fn configure_interface_allows_us_starting_in_memory_terminal() {
        let term_mock = MemoryTerminal::new();
        let term_mock_clone = term_mock.clone();
        let terminal_type = move || -> std::io::Result<MemoryTerminal> { Ok(term_mock_clone) };
        let subject = configure_interface(Box::new(Interface::with_term), Box::new(terminal_type));
        let result = match subject {
            Err(e) => panic!("should have been OK, got Err: {}", e),
            Ok(val) => val,
        };
        let mut wrapper = TerminalWrapper::new(Box::new(result));
        wrapper.lock().write_str("hallelujah").unwrap();

        let checking_if_operational = written_output_all_lines(term_mock.lines(), false);

        assert_eq!(checking_if_operational, "hallelujah");
    }

    #[test]
    fn configure_interface_catches_an_error_when_creating_an_interface_instance() {
        let subject = configure_interface(
            Box::new(producer_of_interface_raw_resulting_in_an_early_error),
            Box::new(result_wrapper_for_in_memory_terminal),
        );

        let result = match subject {
            Err(e) => e,
            Ok(_) => panic!("should have been Err, got Ok with TerminalReal"),
        };

        assert_eq!(
            result,
            format!(
                "Preparing terminal interface: {}",
                Error::from_raw_os_error(1)
            )
        )
    }

    fn producer_of_interface_raw_resulting_in_an_early_error(
        _name: &str,
        _terminal: impl linefeed::Terminal + 'static,
    ) -> std::io::Result<impl InterfaceRaw + Send + Sync + 'static> {
        Err(Error::from_raw_os_error(1)) as std::io::Result<InterfaceRawMock>
    }

    #[test]
    fn configure_interface_catches_an_error_when_setting_the_prompt() {
        let subject = configure_interface(
            Box::new(producer_of_interface_raw_causing_set_prompt_error),
            Box::new(result_wrapper_for_in_memory_terminal),
        );
        let result = match subject {
            Err(e) => e,
            Ok(_) => panic!("should have been Err, got Ok with TerminalReal"),
        };

        assert_eq!(
            result,
            format!("Setting prompt: {}", Error::from_raw_os_error(10))
        )
    }

    fn producer_of_interface_raw_causing_set_prompt_error(
        _name: &str,
        _terminal: impl linefeed::Terminal + 'static,
    ) -> std::io::Result<impl InterfaceRaw + Send + Sync + 'static> {
        Ok(InterfaceRawMock::new().set_prompt_result(Err(Error::from_raw_os_error(10))))
    }

    #[test]
    fn terminal_wrapper_armed_with_terminal_inactive_produces_writer_inactive() {
        let mut subject = TerminalWrapper::new(Box::new(TerminalInactive::default()));

        let lock = subject.lock();

        assert_eq!(lock.tell_me_who_you_are(), "WriterInactive")
    }
}
