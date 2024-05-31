@echo off

set batdir=%~dp0

echo "Building wie-driver-vulkan..."
cargo build || echo "Failed!" && exit 1

echo "Create directories..."
mkdir "C:\Program Files\Vixen\wie\Vulkan\"
echo "Copying ICD file..."
copy "%batdir%vk_wie_icd.json" "C:\Program Files\Vixen\wie\Vulkan\" || echo "Failed!" && exit 1
echo "Copying driver file..."
copy "%batdir%..\..\target\debug\wie_driver_vulkan.dll" "C:\Program Files\Vixen\wie\Vulkan\" || echo "Failed!" && exit 1

echo "Update registry..."
regedit.exe /S "%batdir%driver.reg" || echo "Failed!" && exit 1

echo "Vulkan driver successfully installed."
