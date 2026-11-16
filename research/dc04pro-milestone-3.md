# Mốc 3 — App tray DC04 Pro trên Windows 11 x64

Ngày 16/11/2026. Đã triển khai app tray và đường ghi chung với CLI. Tiêu chí phần cứng của Mốc 3 còn chờ kiểm chứng. Mốc 4 phụ trách đóng gói portable.

## Hành vi

- Bấm icon tai nghe để mở popup Win32 bên cạnh icon. Popup hiển thị tên, trạng thái và nấc DAC. Kéo trackbar, dùng phím mũi tên hoặc cuộn khi trỏ trên trackbar. Esc hoặc bấm sang cửa sổ khác đóng popup. Menu icon có **Đọc lại DAC** và **Thoát**. Đối chiếu [cách neo icon](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shell_notifyicongetrect) và [thông báo trackbar](https://learn.microsoft.com/en-us/windows/win32/controls/wm-hscroll--trackbar) trong tài liệu Microsoft.
- UI hiển thị đích ngay khi chỉnh. Một worker sở hữu HID handle. Trong khi worker đang chạy giao dịch, chỉ giữ một đích chờ; nhả chuột gửi lại đích cuối. Nếu đích đã đúng sau lần đọc trước ghi, không gửi lại chuỗi ghi. Popup mở thì đọc hai thanh ghi định kỳ (350 ms sau khi kết thúc lượt I/O), ưu tiên ghi chờ. Chưa dùng gói nút vật lý làm bằng chứng về nấc.
- Trước mỗi lần ghi, đọc cả hai thanh ghi để kiểm tra balance. Dùng chuỗi 10 report từ [Mốc 2](./dc04pro-milestone-2.md), chỉ báo xác nhận khi đọc lại đúng **cả hai** mã. Khi lỗi hoặc sai mã, worker khóa chỉnh, xóa đích chờ, đóng handle và đọc lại trên handle mới; không tự ghi lại. Lỗi từ một lần mở/đọc được thử lại sau 650 ms.
- Thông báo cắm/rút của lớp HID cho VID/PID 262A:187F vô hiệu hóa phiên ngay trên UI. Worker bỏ đích chờ, dùng mã phiên để loại kết quả cũ và kiểm tra phiên trước mỗi report ghi. Report đã bắt đầu không thể hủy từ UI. Phiên mới luôn đọc hai thanh ghi trước khi nhận đích mới.
- Cặp mã lệch hoặc `00 / 00` vẫn hiện nấc theo cách đọc của CLI, đồng thời khóa chỉnh. Tray và `volume-set` chỉ chỉnh trong 1–99; chế độ thử riêng dùng `bench-edge`. `volume-down [--unplug-test]` giữ hành vi ở Mốc 2.

## Build và chạy lại

```powershell
Set-Location D:\IbassoDC04Pro
cargo fmt --check --manifest-path .\tray\Cargo.toml
cargo test --locked --manifest-path .\tray\Cargo.toml
cargo clippy --locked --all-targets --manifest-path .\tray\Cargo.toml -- -D warnings
cargo build --locked --release --manifest-path .\tray\Cargo.toml
$cli = 'D:\IbassoDC04Pro\tray\target\release\dc04-cli.exe'
$tray = 'D:\IbassoDC04Pro\tray\target\release\dc04-tray.exe'
& $cli list
& $cli get --raw
Start-Process -FilePath $tray
```

## Bài thử phần cứng trên PowerShell Windows thông thường

Dừng phát và tháo tai nghe, nhất là trước bài thử nấc 100. Ngắt web/app điều khiển khác. Nếu balance đang lệch, dùng app iBasso đặt balance giữa rồi ngắt app. Dùng nút vật lý đặt nấc thấp, lớn hơn 1. `get --raw` phải in đúng hai mã bằng nhau và khác 00. Khi đối chiếu từ CLI, đóng popup để dừng đọc định kỳ và chờ UI báo ghi xong trước khi đóng. Mỗi lần gọi CLI bên dưới là tiến trình mới.

| Thao tác | Bằng chứng cần ghi |
| --- | --- |
| Mở popup, nhấn phải rồi trái; trong lượt khác thử cuộn từng nấc | Ghi nấc ban đầu, sau tăng rồi sau giảm bằng `& $cli get --raw`. Hai mã từng lượt phải cùng ứng với đích. Thử Esc và bấm ra ngoài. |
| Kéo nhanh trong một đoạn thấp, nhả ở nấc khác nấc ban đầu | Số phải di chuyển ngay khi kéo. Ghi nấc nhả, chờ trạng thái hết chỉnh, đối chiếu `get --raw` sau khi đóng popup. |
| Khi popup mở và không có thao tác kéo, bấm một nút DAC | Popup tự cập nhật. Đóng popup, đối chiếu cả hai mã bằng CLI. |
| Giữ nút chuột kéo trong đoạn thấp; nhờ người khác rút cáp trước khi nhả | Ghi rõ rút trước/sau nhả, popup đang báo gì và có thấy slider khóa hay không. Khi rút, thử nhả và dùng phím; cắm lại rồi ghi số đầu và số sau vài giây, không chỉnh trong khoảng đó. Đối chiếu CLI, sau đó thử một thao tác mới nếu hai mã khớp. |
| Nếu có thể rút khi popup còn báo “Đang chỉnh âm lượng…” sau khi nhả | Ghi lại diễn biến. Trạng thái này chỉ cho biết app còn đích chưa xác nhận; không chứng minh đã rút đúng lúc một lời gọi HID đang chờ hay đúng giữa hai report cụ thể. |

Bài thử mức biên chạy **riêng**, thoát tray trước. Hai lệnh `bench-edge 0` và `bench-edge 100` chỉ cho ghi từ mức 1–99 có cặp bằng nhau, khác 00. Lệnh `bench-edge 1` chỉ dùng để ra khỏi `FF / FF`. Ghi đích, exit code và kết quả tiến trình mới ở từng bước:

1. **Mức 0:** từ nấc thấp cân giữa, chạy `& $cli bench-edge 0`, rồi `& $cli get --raw` (dự kiến `FF / FF`). Nếu đúng, chạy `& $cli bench-edge 1` rồi `& $cli get --raw` (dự kiến `9B / 9B`). Kiểm tra lại sau khi rút/cắm. Nếu lệch hoặc có lỗi, lưu mã thực và dừng bài thử.
2. **Mức 100:** xác nhận đã ngắt phát và tháo tai nghe. Chạy `& $cli bench-edge 100`, rồi `& $cli get --raw` (dự kiến `00 / 00`). `00 / 00` không chứng minh balance giữa, nên CLI không ghi từ trạng thái này. Dùng **nút giảm của DAC** một nấc, rồi `& $cli get --raw` (cần kiểm tra có về `01 / 01` ở 99 hay không). Nếu cần dùng app iBasso để phục hồi, ghi rõ cách làm và cặp mã sau đó. Khi có cặp bằng nhau khác 00, đưa DAC về nấc thấp và đọc lại trước khi nối tai nghe. Một lần trở về bằng nút vật lý không đủ để mở mức 100 cho tray; cần xác minh đường quay lại an toàn ngay trong app.

## Kết quả

| Kiểm tra | Kết quả |
| --- | --- |
| Format, test, Clippy, build release Windows x64 | Qua, 13 test. Gồm report và đọc lại hai mã, tăng đến đích, từ chối biên, dừng chuỗi khi phiên mất hiệu lực, gộp đích và loại yêu cầu cũ. |
| CLI release trong sandbox | `list` rỗng (exit 0), `get --raw` không thấy collection (exit 1); `volume-set 0` và `volume-set 100` bị từ chối ngay ở bước đọc tham số (exit 2). |
| Đọc ban đầu trên DAC thật | Người dùng chạy CLI release trong PowerShell Windows. `list` thấy một collection `262a:187f`, `000c:0001`, interface 0 (`MI_00`). `get --raw` trả `step=37 reg_0x10=0x4C reg_0x11=0x4A`. Đây là cặp mã đã thấy sau bài rút cáp ở Mốc 2. Chờ kiểm tra khóa slider với chính cặp mã này. |
| Mức thấp sau khi người dùng chỉnh âm lượng | Lần chạy mới của `get --raw` trả `step=22 reg_0x10=0x62 reg_0x11=0x62`. Dùng nấc 22 làm điểm bắt đầu. Theo bảng, tray tăng một nấc thì CLI phải đọc 23 (`61 / 61`), giảm một nấc thì trở về 22 (`62 / 62`). |
| Khởi động trong phiên desktop Windows | Tiến trình tray khởi động và tiếp tục chạy. Công cụ không tìm thấy cửa sổ có thể chọn từ khay, nên chưa quan sát được icon, popup hoặc thao tác đóng mở. Đã đóng tiến trình thử. |
| Nấc biên trên DAC thật | Chờ thử riêng 0 và 100, chưa mở trong slider. |
| Tăng/giảm, kéo nhanh, nút vật lý, phản hồi UI và reconnect với DAC thật | Chờ chạy trong phiên Windows có quyền truy cập HID. |
| Rút cáp thực sự quan sát được ở Mốc 3 | Chưa có. Mốc 2 quan sát rút **sau phản hồi ghi trái `0x13`, trước lần gửi `0x03` tiếp theo**, không phải lúc Windows đang chờ HID. Không tính đó là test tray. |
