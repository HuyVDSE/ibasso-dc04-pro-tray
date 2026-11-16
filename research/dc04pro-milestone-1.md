# Mốc 1 — CLI đọc âm lượng DC04 Pro

Ngày kiểm tra 16/11/2026. Máy thử chạy Windows 11 x64 build 26300.9550, Rust 1.99.1 MSVC x64 và hidapi 2.6.7.

## Chạy lại

```powershell
Set-Location D:\IbassoDC04Pro
cargo test --locked --manifest-path .\tray\Cargo.toml
cargo clippy --locked --all-targets --manifest-path .\tray\Cargo.toml -- -D warnings
cargo build --locked --release --manifest-path .\tray\Cargo.toml
.\tray\target\release\dc04-cli.exe list
.\tray\target\release\dc04-cli.exe get
```

`list` liệt kê collection HID có VID/PID tương ứng. `get` mở đúng một collection, đọc hai thanh ghi qua HID và in nấc 0–100 trên stdout. Lỗi mở, timeout hoặc nhiều thiết bị phù hợp trả mã thoát khác 0. Chạy trên PowerShell Windows thông thường. Trong sandbox của Codex, PnP liệt kê được thiết bị nhưng hidapi không thấy collection. Kiểm thử HID trực tiếp dùng tiến trình ngoài sandbox.

## Thiết bị và report

| Thuộc tính | Quan sát |
| --- | --- |
| Tên do hidapi trả về | `iBasso-DC04-Pro` |
| VID/PID | `262A:187F` |
| Collection | `MI_00`, interface `0`, usage page `000C`, usage `0001` trong cả PnP và hidapi |
| Descriptor đọc qua hidapi | Output payload 16 byte, Input payload 32 byte, report không đánh số, không có Feature Report |
| FPGA | `1.58`. Input cho lệnh `0x58` có payload bắt đầu `00 00 00 00 00 00 58 01 58 5C`. Byte dữ liệu `[8]` là `0x58` theo bộ phân tích APK |

Hai truy vấn dùng Output Report ID `0` với payload sau. Khi gọi `HidDevice::write`, thêm byte `00` đầu làm report ID cho HIDAPI. Backend Windows bỏ byte ID `0` khi trả Input Report từ `read_timeout`.

```text
Thanh ghi 0x10: 41 12 E4 A2 00 10 01 00 00 00 00 00 00 00 00 00
Thanh ghi 0x11: 41 12 E4 A2 00 11 01 00 00 00 00 00 00 00 00 00
```

Phản hồi gồm 32 byte payload. Offset tính từ 0. Byte `[6]` là mã truy vấn `0x41`, `[7]` là độ dài trường dữ liệu `1`, `[8]` là mã suy hao. Report nút vật lý mà APK nhận diện bằng `[4..6] = FE 01` bị bỏ qua trong lúc chờ phản hồi truy vấn. CLI truy vấn từng thanh ghi tuần tự với hạn chờ phản hồi 500 ms cho mỗi truy vấn. HIDAPI C trên Windows đặt hạn chờ cho thao tác gửi là 1 giây. Chưa đo các giới hạn này khi rút cáp giữa giao dịch. [Tài liệu HIDAPI](https://docs.rs/hidapi/2.6.7/hidapi/struct.HidDevice.html)

CLI chọn mã lớn hơn trong hai mã rồi ánh xạ qua bảng 101 nấc. Cách chọn này dựa trên thuật toán balance của hãng, giữ mã gốc ở một kênh và giảm mã ở kênh còn lại. Trường hợp nằm giữa hai mã bảng được làm tròn theo cách APK dùng. Chưa kiểm thử trường hợp balance khác 0 trên máy thật.

## Nguồn giao thức

- [Bundle chính của bản web local](../local-web/assets/index-DLTNBGaH.js), SHA-256 `687C7107FFB68B95385661D3928C3833BD36110855212B6E3774D9D2F18031EE`. `yd` chọn controller `_n` cho DC04 Pro. `_n.initialize` gọi `Pt.initialize` để gán volume mặc định `50`, rồi gửi lệnh FPGA `0x58`. `Pt.parseResponseData` chỉ ghi log. `It.handleInputReport` dùng byte `[6]` làm mã để ghép phản hồi 32 byte có report ID 0. `Pt.VOLUME_TABLE` là bảng đổi nấc sang mã suy hao.
- [View DC04 Pro](../local-web/assets/Dc04ProView-CeT831J1.js) đọc `deviceParams.volume`, rồi `volumeLeft`, rồi mặc định `50`. Bản website trực tuyến ngày 16/11 dùng [index-CBVDh9A1.js](https://devices.ibasso.cn/assets/index-CBVDh9A1.js), SHA-256 `493441A69C6E734D7D62A26B11B1B7B6F2CA984A7E32AA08EAD133DA414032FC`, và [Dc04ProView-CwjSgdTN.js](https://devices.ibasso.cn/assets/Dc04ProView-CwjSgdTN.js). Các controller `Is` và `Rt` vẫn khởi tạo volume 50, truy vấn FPGA và chỉ ghi log cho phản hồi ngoài truy vấn. Bảng dưới ghi số quan sát thực tế trong tab trực tuyến trên Thorium.
- App iBasso UAC 1.4.4 tải từ iBasso, SHA-256 `F1662EA04C308290255ED2CD58427F2834FE75D1622F4AE34BADCD77527393BD`. Phân tích tĩnh bằng JADX 1.5.6. `j3/c.java` (`Dc04ProDeviceController`) đưa thanh ghi `0x10` hoặc `0x11` vào yêu cầu `0x41` tùy balance. `h3/s0.java` dựng payload, gửi USB SET_REPORT Output và đọc byte `[8]` từ phản hồi. `j3/c.java` và `j3/h.java` chứa cách đổi mã suy hao sang nấc hiển thị. Chỉ lấy hai truy vấn đọc từ đường này. [Trang tải của hãng](https://ibasso.com/dc0304pro/)
- REA phân tích tĩnh có giới hạn ba tệp web. Evidence cha `ev_3a56d69fe04a0c7410b1a99ad4558e1a3482d63543c30411ef131e0299503c1d`.

## Kết quả kiểm tra

- `cargo fmt --check`, hai test giải mã, Clippy với `-D warnings` và build release đều qua.
- `list` tìm thấy đúng một collection. Mỗi lần chạy `get` mở HID handle mới và gửi lại hai truy vấn.
- Sau khi cắm lại, Core Audio xác nhận thiết bị phát mặc định ở role Console và Multimedia là `Speakers (Realtek(R) Audio)`. CLI vẫn trả `37` khi web đã kết nối lại.

| Thao tác tại máy | CLI release | Mã HID ở byte `[8]` của hai phản hồi | Tab iBasso |
| --- | --- | --- | --- |
| Lần đọc ban đầu | `28`, exit `0` | `5C / 5C` | FPGA `1.58`, slider `50` |
| Sau khi người dùng bấm nút tăng, 09:34:41 | `37`, exit `0` | `4C / 4C` | Người dùng báo slider không đổi ngay khi bấm. Lần quan sát sau đó thấy `37` |
| Sau khi rút cáp, 09:35:39 | Exit `1`, không tìm thấy DAC. `list` cũng rỗng | — | Web báo mất kết nối |
| Lần đọc đầu sau khi cắm lại, 09:35:51 | `37`, exit `0` | Lần đọc lại lúc 09:36:08 thấy `4C / 4C` | Web vẫn ngắt kết nối |
| Sau khi kết nối lại web, 09:37:37 | `37`, exit `0` | — | Slider trở về `50` |

Các phản hồi volume quan sát được đều dài 32 byte. Phần đầu lần thứ nhất là `00 00 00 00 00 00 41 01 5C 5C`. Sau khi bấm là `00 00 00 00 00 00 41 01 4C 4C`, và sau khi cắm lại là `00 00 00 00 00 00 41 01 4C 00`. Các byte còn lại đều bằng 0 trong các mẫu này.

Đã xác minh hai nấc khác nhau trực tiếp qua HID và đọc lại thành công sau cấp nguồn. Đối chiếu web khớp ở một lần quan sát. Web hiển thị `50` ở hai thời điểm CLI trả giá trị khác, nhất quán với việc controller khởi tạo slider về `50`. Vì vậy, tiêu chí hai nấc cùng khớp trên web chưa đạt. Chưa đo sự kiện HID khi bấm nút, toàn bộ dải nấc, balance khác 0 hoặc thời gian I/O khi rút giữa truy vấn.
