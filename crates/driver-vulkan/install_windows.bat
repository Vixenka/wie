set batdir=%~dp0

echo "Building wie-driver-vulkan..."
cargo build --lib wie-driver-vulkan --release

echo "Create directories..."
mkdir "C:\Program Files\Vixen\wie\Vulkan\"
echo "Copying ICD file..."
copy "%batdir%\vk_wie_icd.json" "C:\Program Files\Vixen\wie\Vulkan\"
echo "Copying driver file..."
copy "%batdir%\..\..\target\release\wie_driver_vulkan.dll" "C:\Program Files\Vixen\wie\Vulkan\"

echo "Update registry..."
reg add "HKEY_LOCAL_MACHINE\SOFTWARE\Khronos\Vulkan\Drivers" /v "C:\Program Files\Vixen\wie\Vulkan\vk_wie_icd.json" /t REG_DWORD /d 0 /f

echo "Vulkan driver successfully installed."
