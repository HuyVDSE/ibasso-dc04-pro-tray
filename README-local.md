# iBasso chạy local

Bản sao các tài nguyên frontend công khai của https://devices.ibasso.cn/, tải ngày 27/10/2026. Đây là bản build JavaScript/CSS đã phát hành, không phải source Vue/TypeScript gốc hoặc bản sao backend.

## Mở ứng dụng

Trong PowerShell:

```powershell
cd D:\IbassoDC04Pro
.\Start-Local.ps1
```

Mở http://127.0.0.1:8044/ bằng Chrome hoặc Edge trên máy đang cắm DAC. Chọn **Connect Device**, chọn **iBasso-DC04-Pro**, rồi cấp quyền kết nối. Quyền của website iBasso không được dùng chung với địa chỉ local. Đóng kết nối ở tab iBasso khác nếu thiết bị đang bị giữ.

Server chỉ lắng nghe 127.0.0.1, phục vụ thư mục `local-web`, không phục vụ toàn ổ đĩa. Dữ liệu điều khiển DAC đi qua WebHID của trình duyệt. HTTP loopback được các trình duyệt hỗ trợ xem là ngữ cảnh đáng tin cậy; không cần cài chứng chỉ hay sửa cờ bảo mật. Không mở index.html bằng file://.

Để dừng server:

```powershell
.\Stop-Local.ps1
```

Hoặc chạy server trực tiếp trong terminal (Ctrl+C để dừng):

```powershell
python .\serve_local.py
```

## Phạm vi

- Đã tải các bundle, CSS và ảnh giao diện về máy; không cần npm/build.
- Giữ nguyên bundle điều khiển DAC của hãng.
- Server gửi CSP `connect-src 'self'` để chặn fetch/XHR/WebSocket tới dịch vụ bên ngoài. Không tạo proxy tới backend iBasso.
- Đăng nhập, OAuth, tài khoản và cộng đồng preset vẫn có nút trong frontend gốc nhưng không được hỗ trợ trong bản local. Không sử dụng các nút đó để đăng nhập. Trang OAuth ngoài vẫn có thể mở nếu chủ động nhấn đăng nhập.
- Bản local không tự cập nhật. Script mirror_ibasso.py có thể dùng để tải lại; nó tái sử dụng tệp đã có, vì vậy muốn cập nhật cùng URL cần quản lý snapshot riêng.
- Đây là bản sao dùng cá nhân để chạy tại máy. Các tài nguyên của iBasso vẫn thuộc chủ sở hữu ban đầu.

## Kiểm tra

- Python đã kiểm tra cú pháp.
- Đã mở trang chủ từ 127.0.0.1 và thấy giao diện Connect Device.
- Người dùng đã chọn/cấp quyền WebHID cho origin local. Đã xác nhận bản local vào `#/device/DC04PRO`, đọc DC04 PRO / FPGA Version 1.58 / volume 50 và hiện gain, output, Eco, filter, balance. Ảnh xác nhận: `research/ibasso-local-connected.png`.
- Chưa thử lệnh ghi hoặc thay đổi thông số DAC, chưa xác nhận offline bằng ngắt mạng thực tế. Kết nối và đọc trạng thái hoạt động với CSP chặn API bên ngoài.
- Website gốc trong phiên trước đã nhận DC04 PRO / FPGA 1.58. Kết quả đó không được coi là bằng chứng kết nối của bản local.
- `mirror-manifest.json` lưu URL nguồn, đường dẫn, kích thước và SHA-256 các tệp tải được, cùng những đường dẫn tải lỗi.

## Các tệp

- `local-web/`: frontend đã tải.
- `serve_local.py`: server Python không cần thư viện ngoài.
- `Start-Local.ps1`, `Stop-Local.ps1`: mở/dừng server có kiểm tra PID.
- `mirror_ibasso.py`: script tải tài nguyên tĩnh công khai.
- `local-server-error.log`: access log và lỗi server.
- `research/`: ảnh và báo cáo.
