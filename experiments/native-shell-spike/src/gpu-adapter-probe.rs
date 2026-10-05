use eframe::wgpu;

fn main() {
    assert_eq!(
        std::env::args_os().count(),
        1,
        "adapter probe accepts no arguments"
    );
    let backends = wgpu::Instance::enabled_backend_features();
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapters = pollster::block_on(instance.enumerate_adapters(backends));
    let mut cpu_count = 0;
    println!("compiled_backends={backends:?}");
    println!("adapter_count={}", adapters.len());
    for adapter in &adapters {
        let info = adapter.get_info();
        if info.device_type == wgpu::DeviceType::Cpu {
            cpu_count += 1;
        }
        println!(
            "adapter_name={} backend={:?} device_type={:?}",
            info.name, info.backend, info.device_type
        );
    }
    println!("cpu_adapter_count={cpu_count}");
    let fallback = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        force_fallback_adapter: true,
        ..Default::default()
    }));
    match fallback {
        Ok(adapter) => {
            let info = adapter.get_info();
            println!(
                "fallback_available=true fallback_name={} fallback_backend={:?} fallback_device_type={:?}",
                info.name, info.backend, info.device_type
            );
        }
        Err(_) => println!("fallback_available=false"),
    }
    println!("window_surface_tested=false device_loss_tested=false renderer_recovery_tested=false");
}
