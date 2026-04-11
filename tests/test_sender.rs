use relm4::factory::*;
fn test(sender: FactorySender<()>) {
    let _ = sender.send(());
}
