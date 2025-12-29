# Kernel Performance Benchmarks

This directory contains performance benchmarks comparing the kernel implementation against the monolith to validate the microkernel architecture.

## Running Benchmarks

```powershell
# Run all benchmarks
cargo bench -p raps-kernel

# Run specific benchmark group
cargo bench -p raps-kernel -- config_loading
cargo bench -p raps-kernel -- type_validation
cargo bench -p raps-kernel -- http_client
cargo bench -p raps-kernel -- error_handling
cargo bench -p raps-kernel -- auth_client
cargo bench -p raps-kernel -- url_helpers
cargo bench -p raps-kernel -- memory_footprint
```

## Benchmark Groups

### `config_loading`
Measures configuration loading from environment variables:
- `kernel_config_from_env`: Time to load Config from environment

### `type_validation`
Measures type validation performance:
- `bucket_key_parse`: BucketKey validation
- `object_key_parse`: ObjectKey creation
- `urn_from_str`: URN creation from string

### `http_client`
Measures HTTP client creation:
- `kernel_http_client_new`: HttpClient instantiation

### `error_handling`
Measures error creation and conversion:
- `kernel_error_creation`: RapsError creation
- `kernel_error_from_io`: IO error conversion

### `auth_client`
Measures authentication client creation:
- `kernel_auth_client_new`: AuthClient instantiation

### `url_helpers`
Measures URL helper method performance:
- `kernel_auth_url`: Auth endpoint URL generation
- `kernel_oss_url`: OSS endpoint URL generation
- `kernel_derivative_url`: Derivative endpoint URL generation
- `kernel_project_url`: Project endpoint URL generation
- `kernel_data_url`: Data endpoint URL generation

### `memory_footprint`
Measures memory usage of key types:
- `bucket_key_size`: Size of BucketKey
- `object_key_size`: Size of ObjectKey
- `http_client_size`: Size of HttpClient

## Expected Results

The kernel should show:
- **Faster initialization**: Minimal overhead from microkernel design
- **Lower memory footprint**: Smaller types, less indirection
- **Type safety**: Validation overhead is minimal (< 1μs per operation)
- **Consistent performance**: No performance regressions vs monolith

## Viewing Results

Results are saved to `target/criterion/` and can be viewed in HTML format:

```powershell
# Open the HTML report (Windows)
Start-Process target\criterion\config_loading\kernel_config_from_env\report\index.html
```

## Comparison with Monolith

To compare with the monolith implementation, you would need to:
1. Add benchmarks to the `raps` crate
2. Run both benchmark suites
3. Compare results using Criterion's comparison features

The kernel benchmarks establish a baseline for future comparisons.
