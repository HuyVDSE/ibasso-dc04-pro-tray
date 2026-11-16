# Mốc 4 — iBasso DC04 Pro Tray portable

Ngày 16/11/2026 · Windows 11 x64 (build 26300) · Rust 1.99.1, MSVC.

**File bàn giao:** [`D:\IbassoDC04Pro\dist\dc04-tray.exe`](../dist/dc04-tray.exe), 442.368 byte.  
**SHA-256:** `4B1EB96D3D3D09DC15A7E430357195F9D12CC1C40164B158349568148C5414EA`

## Chạy và di chuyển

Mở trực tiếp `dist\dc04-tray.exe` bằng tài khoản Windows thông thường. Chỉ cần file EXE, không cần cài đặt hay chép icon/manifest theo. Tìm biểu tượng tai nghe trong khay hệ thống, kể cả trong vùng biểu tượng ẩn dưới dấu **^**. Bấm trái để mở popup, bấm phải để mở menu. App giữ nguyên giao thức HID ở Mốc 3 và chỉ cho chỉnh **1–99**. Người dùng đã xác nhận các bài thử phần cứng Mốc 3 qua sau khi báo cáo Mốc 3 được viết.

Trong menu, **Khởi động cùng Windows** có dấu chọn khi mục `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\IbassoDc04ProTray` trỏ tới đúng EXE đang chạy. Bật lưu đường dẫn tuyệt đối trong cặp dấu ngoặc kép, dạng `"D:\Thư mục có dấu cách\dc04-tray.exe"` (`REG_SZ`); tắt xóa đúng mục này. Nếu di chuyển EXE, chọn **Thoát** trước; mở file ở vị trí mới rồi chọn **Khởi động cùng Windows (cập nhật đường dẫn)**. Có thể chọn **Gỡ đăng ký đường dẫn cũ** nếu muốn tắt hẳn. Menu phản ánh đăng ký của app; nếu đã tắt app qua **Settings → Apps → Startup**, kiểm tra lại trạng thái trong Windows khi cần cho phép chạy lúc đăng nhập.

Khi Explorer dựng lại taskbar, tray đăng ký lại icon. Sau khi Windows thức dậy, app yêu cầu worker bỏ phiên cũ và đọc mới qua luồng đã kiểm chứng. Nếu web app cùng dùng HID và gặp sự cố, menu **Chi tiết lỗi DAC** hiện nguyên văn lỗi gần nhất của tray; mục này biến mất sau khi tray đọc lại thành công.

## Kết quả trên file bàn giao

| Kiểm tra | Kết quả / giới hạn |
| --- | --- |
| Format, test, Clippy, release | **Qua**. `cargo fmt --check`, `cargo test --offline --locked` (13 test), `cargo clippy --offline --locked --all-targets -- -D warnings`, `cargo build --offline --locked --release`, đều dùng `--manifest-path tray/Cargo.toml` từ workspace. |
| Tài nguyên và phụ thuộc | **Qua**. PE x64 GUI, manifest ID 1: `PerMonitorV2`, `asInvoker`, Common Controls 6; icon nhóm 1 chứa 5 cỡ 16/24/32/48/64 px. Windows SDK `mt.exe` xác thực manifest trích từ release. Runtime C liên kết tĩnh; không thấy thư viện runtime Visual C++ trong bảng import. |
| Portable với đường dẫn có dấu cách và chỉ một instance | **Qua trên SHA-256 bàn giao**. Smoke test từ `Portable Test A`, working directory `research`, không nâng quyền: PID 12360 phiên 1 tiếp tục chạy; bản giống hệt từ `Portable Test B` thoát mã 0. Đã dọn tiến trình thử. Trong phiên desktop, người dùng xác nhận icon hiện và mở popup đọc được DAC ở nấc **33**. |
| Explorer restart | **Qua theo quan sát của người dùng**: sau khi restart Windows Explorer, vẫn thấy icon và mở popup được. Không ghi PID desktop trước/sau. |
| Sleep/resume | Người dùng xác nhận chỉnh được âm lượng sau khi máy thức dậy. Chưa có số nấc hiện ngay sau khi thức và trước khi chỉnh. |
| DPI 100% / 150% | **Qua theo bài thử của người dùng**. Popup neo cạnh icon, nằm trong vùng làm việc và điều khiển dùng được ở cả hai mức. |
| Web iBasso và tray cùng kết nối HID, sau đó ngắt web | Người dùng báo mức âm lượng vẫn là **33** và không báo lỗi mở hoặc đọc DAC. Người dùng nghi web nhận dữ liệu nhưng số trên giao diện không tự đổi. Chưa có log HID từ trình duyệt và kết quả riêng của tray cho lúc web đang kết nối và sau khi ngắt web. |
| Startup bật/tắt, EXE di chuyển, đăng nhập và khôi phục | **Qua theo bài thử của người dùng**, gồm bật/tắt mục đăng ký, cập nhật đường dẫn từ A sang B, tự mở tray sau khi đăng nhập lại và khôi phục cấu hình ban đầu. |

Trong bản web local, giao diện DC04 Pro theo dõi `deviceParams.volume` và `volumeLeft`. Controller gán nấc mặc định **50** khi kết nối rồi truy vấn phiên bản FPGA. Thao tác chỉnh âm lượng ngay trên web cập nhật giá trị trong ứng dụng. Report HID đến mà không khớp truy vấn đang chờ được chuyển tới `parseResponseData`, nơi controller DC04 Pro chỉ ghi log. Vì vậy, nếu trình duyệt nhận report khi nấc thay đổi bên ngoài web, riêng luồng này không đưa nấc mới lên slider. [View DC04 Pro](../local-web/assets/Dc04ProView-CeT831J1.js) · [Bundle controller](../local-web/assets/index-DLTNBGaH.js) · [Đối chiếu trên DAC trong Mốc 1](./dc04pro-milestone-1.md)

## Bài thử có DAC trong phiên desktop

Thoát bản tray cũ, dừng phát, tháo tai nghe và đặt DAC ở nấc thấp, cân hai kênh. Chạy bằng PowerShell **không** mở bằng quyền quản trị. Hai bản thử trong thư mục có dấu cách chỉ chứa bản sao của EXE bàn giao.

```powershell
$test = 'D:\IbassoDC04Pro\research\test-dc04pro-portable.ps1'
& $test -Action Launch
```

Lệnh kiểm tra SHA-256 bản sao, chạy từ thư mục làm việc khác, kiểm tra lần chạy thứ hai thoát mã 0, rồi để instance đầu chạy cho các bài dưới đây. Ghi PID, icon có xuất hiện hay không, popup và nấc ban đầu.

1. **Explorer:** trong Task Manager, chọn **Windows Explorer → Restart**. Tìm icon tai nghe, kể cả dưới dấu **^**, mở popup, ghi nấc và so PID với ban đầu (ví dụ `Get-Process dc04-tray | Select-Object Id,SessionId,Path`).
2. **Sleep/resume:** ghi nấc rồi chọn **Start → Power → Sleep**, đánh thức máy, mở popup và ghi nấc. Nhấn mũi tên tăng hoặc giảm một nấc, chờ app xác nhận, ghi kết quả rồi phục hồi nấc ban đầu. Ghi dòng trạng thái nếu slider vẫn bị khóa.
3. **DPI:** trong **Settings → System → Display → Scale**, thử lần lượt 100% và 150%; mỗi lần mở lại popup, kiểm tra vị trí bên cạnh icon, không bị cắt và thử chỉnh một nấc tại mức thấp. Phục hồi nấc và cấu hình Scale ban đầu.
4. **Web/HID:** dùng [bản web iBasso local](../README-local.md), chọn **Connect Device** và xác nhận web thấy đúng DAC. Khi tray đã chạy, mở popup rồi ghi trạng thái của hai app. Từ menu tray chọn **Đọc lại DAC** khi web vẫn kết nối để ép mở handle mới; mở lại popup và ghi nấc hoặc nguyên văn **Chi tiết lỗi DAC**, gồm lỗi mở/đọc nếu có. Ngắt kết nối web hoặc đóng tab đang giữ HID, chọn **Đọc lại DAC**, ghi tray có phục hồi và đọc đúng nấc hay không. Nếu thử lại theo thứ tự web trước, tray sau, ghi kết quả riêng. Không suy ra nguyên nhân chỉ từ việc một app lỗi.
5. **Startup:** làm theo trình tự bên dưới. Đăng xuất là bước cuối để không làm mất trạng thái các bài khác.

### Kiểm tra và khôi phục startup

1. Trước khi bật/tắt, chạy `& $test -Action Baseline | Format-List` để lưu hai giá trị **riêng của app**: Run và trạng thái tương ứng trong StartupApproved. Bản sao lưu nằm cạnh script và dùng được sau khi đăng xuất. Nếu sao lưu lỗi, dừng phần startup.
2. Bật qua menu từ `Portable Test A`, mở lại menu xem dấu chọn và chạy `& $test -Action Show | Format-List`. Run phải xuất hiện dạng `REG_SZ` với cả hai dấu ngoặc kép. Tắt, mở menu và chạy lại **Show**: mục Run phải vắng mặt.
3. Bật lại từ A, **Thoát**, đổi tên tạm thư mục `Portable Test A` thành `Portable Test A old` rồi mở EXE giống hệt trong `Portable Test B`. Mục startup phải bỏ dấu chọn và báo cập nhật đường dẫn. Bấm để sửa đăng ký, kiểm tra dấu chọn và đường dẫn B bằng **Show**.
4. Lưu công việc, **Sign out** rồi đăng nhập lại. Đợi Windows nạp ứng dụng startup; xác nhận tray xuất hiện và tiến trình chạy từ B. Sau đó tắt trong menu và xác nhận Run đã được gỡ bằng **Show**. Nếu không tự mở được, ghi trạng thái Startup trong Windows và giá trị Run lúc đó.
5. Dù kết quả thế nào, trả lại tên thư mục A, chạy `& $test -Action Restore | Format-List` từ cùng tài khoản và đối chiếu với kết quả **Baseline**. Chỉ khi đối chiếu xong mới xóa bản sao lưu nếu muốn chạy lại từ đầu.
