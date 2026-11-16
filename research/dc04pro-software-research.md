# Nghiên cứu điều khiển iBasso DC04 Pro

Ngày kiểm tra: 27/10/2026.

## Kết luận

Có bảng điều khiển web chính thức tại https://devices.ibasso.cn/ . Trang sản phẩm của iBasso Trung Quốc liên kết trực tiếp đến bảng điều khiển này. Không cần tự xây ứng dụng chỉ để có giao diện điều khiển trên máy tính.

## Đã kiểm chứng trên máy hiện tại

- Windows nhận thiết bị `iBasso-DC04-Pro`, USB VID 262A, PID 187F, audio interface MI_01.
- Mở web app và chọn Connect Device đã vào được trang `https://devices.ibasso.cn/#/device/DC04PRO`.
- Giao diện nhận DC04 PRO, hiển thị FPGA Version 1.58 và volume 50.
- Các mục điều khiển xuất hiện: volume; Gain Level Low/Mid/High; Output Mode Normal/Boost; Eco Mode; Digital Filter Fast Roll-off, LD Slow Roll-off, LD Fast Roll-off, Slow Roll-off, NOS; Channel Balance.
- Không chỉnh volume, gain, filter hoặc chế độ đầu ra. Đã xác minh kết nối/nhận diện và các mục giao diện, chưa kiểm thử tác dụng của từng lệnh ghi hay khả năng lưu cấu hình sau khi rút cáp.
- Ảnh: ibasso-web-control.png.

## Phần mềm khác của hãng

- Android: iBasso UAC APK. Trang hỗ trợ Trung Quốc liệt kê bản V2.1.9 ngày 12/08/2026; lịch sử ghi nhận hỗ trợ DC04PRO từ V1.4.4.
- Windows: hãng cung cấp USB-DAC driver. Driver và ứng dụng điều khiển là hai thành phần khác nhau; việc có driver không chứng minh có đầy đủ giao diện gain/filter.
- Trang tải quốc tế dành cho DC04Pro vẫn liệt kê Android app V1.8.6 và Windows driver V3.13.047, cũ hơn danh mục APK Trung Quốc.
- Chưa xác định được ứng dụng desktop native riêng cho DC04 Pro từ nguồn chính thức đã kiểm tra. Web app chính thức là lựa chọn trực tiếp đã kết nối được trong phiên này.

## Khi nào nên tự build

Chỉ đáng cân nhắc nếu cần chạy offline, system tray, phím tắt, preset hoặc tự áp cấu hình khi cắm DAC. Trước khi chọn thư viện/kiến trúc cần xác định interface điều khiển và giao thức thực tế từ web app/APK; không suy ra từ audio interface MI_01 rằng điều khiển dùng cùng interface. Cần đối chiếu lệnh đọc/ghi và readback trước khi triển khai giao diện. Chưa phân tích giao thức hay tạo mã điều khiển trong đợt nghiên cứu này.

## Nguồn

- https://devices.ibasso.cn/
- https://ibasso.cn/product/dc04pro
- https://ibasso.cn/support/dcapklist
- https://ibasso.com/dc0304pro/
- https://ibasso.cn/support/dcpcdrivera
