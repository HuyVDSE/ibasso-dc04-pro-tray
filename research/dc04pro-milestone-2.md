# Mốc 2 — CLI giảm một nấc âm lượng DC04 Pro

Ngày 16/11/2026. **Đạt Mốc 2.** Trên DAC thật, tiến trình CLI mới xác nhận giảm đúng một nấc với cả hai mã đích. Bài thử rút cáp sau phản hồi ghi trái cho thấy CLI báo lỗi và một thanh ghi giữ mã mới sau khi cắm lại. Kiểm tra tự động chạy trên Windows x64, Rust 1.99.1 MSVC và hidapi 2.6.7.

## Đối chiếu đường ghi

- [Bundle web local](../local-web/assets/index-DLTNBGaH.js), SHA-256 `687C7107FFB68B95385661D3928C3833BD36110855212B6E3774D9D2F18031EE`. Factory `yd` chọn `_n` kế thừa `Pt` cho DC04 Pro. Đối chiếu `Pt.setVolume`, `computeChannelHw`, `writeChannelVolumes`, `buildReg`, `buildReg2` và transport `It.sendOutputReportWithCmdResponse` / `handleInputReport`. REA phân tích tĩnh hai tệp web, Evidence cha `ev_4f37f9fb9088b25749fd84e906d0e8b6c32cde07760ee77aad971a99ddeb750f`.
- [Bundle web trực tuyến của Mốc 1](https://devices.ibasso.cn/assets/index-CBVDh9A1.js), bản đã tải khi làm Mốc 1, SHA-256 `493441A69C6E734D7D62A26B11B1B7B6F2CA984A7E32AA08EAD133DA414032FC`. Factory chọn `Is` kế thừa `Rt`; bảng nấc, tính balance, 10 payload và thứ tự phía web giống bản local.
- [iBasso UAC APK 1.4.4](https://ibasso.com/dc0304pro/), SHA-256 `F1662EA04C308290255ED2CD58427F2834FE75D1622F4AE34BADCD77527393BD`. Đối chiếu bản giải mã tĩnh JADX 1.5.6 từ Mốc 1: `j3/c.java` (`q`, `z`), `j3/h.java` (bảng nấc), `h3/t.java` (hằng số), `h3/r0.java` (kiểu tác vụ), `h3/s0.java` (`u`, `v`, `b`). Ở balance giữa APK gửi cùng 10 payload nhưng xen hai lệnh đầu cho phải sớm hơn: `01 02 03 04 09 0A 13 0B 0C 14`, sau đó truy vấn `0x41`. Khi balance lệch, APK đổi một số mã lệnh theo nhánh. Cả APK và web đều dùng `0x13` cho thanh ghi `0x10` bên trái trước `0x14` cho `0x11` bên phải.

CLI dùng đầy đủ thứ tự của web. Output report ID `0` có payload 16 byte. Bảng dưới dùng `L` và `R` làm mã suy hao trái/phải dạng byte; HIDAPI nhận thêm byte ID `00` trước các byte này.

```text
01 11 88 60 00 00 05 09 00 01 00 L  00 00 00 00
02 11 88 60 00 00 05 09 00 02 00 L  00 00 00 00
09 11 88 60 00 00 05 07 00 00 00 L  00 00 00 00
0A 11 88 60 00 00 05 07 00 01 00 L  00 00 00 00
13 11 A0 A2 00 10 01 L  00 00 00 00 00 00 00 00
03 11 88 62 00 00 05 09 00 01 00 R  00 00 00 00
04 11 88 62 00 00 05 09 00 02 00 R  00 00 00 00
0B 11 88 62 00 00 05 07 00 00 00 R  00 00 00 00
0C 11 88 62 00 00 05 07 00 01 00 R  00 00 00 00
14 11 A0 A2 00 11 01 R  00 00 00 00 00 00 00 00
```

Bảng 101 nấc ánh xạ 0 → `FF`, 1 → `9B`, 28 → `5C`, 100 → `00`. Mã suy hao tăng khi giảm nấc; chênh lệch mã giữa hai nấc không cố định. Ở balance giữa (web `10`, APK `0`), cả hai kênh dùng cùng một mã bảng. Web trừ độ lệch tuyệt đối khỏi mã một kênh (giá trị dương ứng với kênh phải), chặn ở 0 và giữ nguyên mã gốc ở kênh kia. APK cũng trừ độ lệch, nhưng tại nấc 0 giữ cả hai mã `FF`. Hai mã lệch có thể là balance lệch hoặc dấu vết một lần ghi chưa xong; cặp `00 / 00` cũng không tiết lộ balance do chặn ở 0. Vì vậy `volume-down` chỉ ghi sau khi đọc được hai mã bằng nhau, khác `00`, và nấc lớn hơn 0. `get` vẫn dùng quy tắc làm tròn của Mốc 1 kể cả khi hai mã khác nhau.

Web chờ phản hồi từng lệnh, mặc định 500 ms. Với input không đánh số, dài 32 byte, byte payload `[6]` là mã lệnh; web không dùng dữ liệu trả về để kiểm tra mức đã ghi. APK cũng ghép theo `[6]`, bỏ qua sự kiện nút vật lý `FE 01` ở `[4..6]` và không đọc mã suy hao từ phản hồi lệnh ghi. CLI đợi gói khớp theo quy tắc này, rồi truy vấn `0x41` cho từng thanh ghi và so sánh **cả hai mã** với đích. Phản hồi ghi chỉ cho biết có gói khớp lệnh; chưa bắt gói ghi thực trên DAC để xác định các byte khác. Không dùng slider web hoặc trạng thái khởi tạo 50 làm bằng chứng.

CLI dừng ở lỗi gửi, thiếu phản hồi hoặc đọc lại không khớp sau khi bắt đầu ghi. Nó báo `Volume outcome unknown`, không tự ghi lại hay hoàn tác. Nếu lỗi xảy ra trước ghi, CLI báo không có lần thử ghi âm lượng. Mỗi lần gọi tiếp theo mở HID handle mới và đọc lại cả hai thanh ghi trước khi gửi bất kỳ report ghi âm lượng nào. Hạn chờ cấu hình là **500 ms mỗi lần chờ phản hồi** sau khi gửi; backend HIDAPI C trên Windows đặt **1.000 ms** cho lần đợi hoàn tất một output bất đồng bộ. Đây là giới hạn từng I/O, không phải thời gian đo được khi rút cáp hay hạn chót chung của CLI.

## Chạy lại

```powershell
Set-Location D:\IbassoDC04Pro
cargo fmt --check --manifest-path .\tray\Cargo.toml
cargo test --locked --manifest-path .\tray\Cargo.toml
cargo clippy --locked --all-targets --manifest-path .\tray\Cargo.toml -- -D warnings
cargo build --locked --release --manifest-path .\tray\Cargo.toml
```

Chạy phần sau trên PowerShell Windows ngoài sandbox. Dừng phát và tháo tai nghe trước khi thử. Tắt kết nối web/app điều khiển khác. Nếu cần chỉnh, đặt balance giữa trong app iBasso rồi ngắt app, dùng nút DAC để đặt một nấc thấp lớn hơn 1. Không bấm nút hay đổi balance trong lúc CLI chạy.

```powershell
$cli = 'D:\IbassoDC04Pro\tray\target\release\dc04-cli.exe'
& $cli list
& $cli get --raw
```

Ghi lại dòng đọc đầu tiên. Chỉ tiếp tục khi thấy đúng một DAC, nấc thấp và hai mã bằng nhau. Chạy lệnh giảm đúng một lần, sau đó hai lần đọc là hai tiến trình mới.

```powershell
& $cli volume-down
"write_exit=$LASTEXITCODE"
& $cli get
& $cli get --raw
```

Nếu báo chưa xác định, giữ nguyên trạng thái, lưu lỗi và dùng `get --raw` để đồng bộ trước khi cân nhắc thao tác khác. Kết quả cần nấc mới đúng bằng nấc ban đầu trừ 1 và cả hai mã bằng mã bảng của nấc mới. Ví dụ dự kiến nếu bắt đầu ở nấc 28, `5C / 5C` phải thành nấc 27, `5D / 5D`; đây chỉ là ví dụ từ bảng và test tự động.

Thử rút cáp riêng sau khi xác nhận bài thử bình thường. Đọc trạng thái trước. Chỉ rút sau thông báo `Received reply for 0x13` và giữ rút đến khi CLI thoát. Tùy chọn thử chèn đúng một khoảng chờ cấu hình 5 giây giữa phản hồi ghi trái và lần gửi cho phải. Phép thử kiểm tra mất kết nối giữa các bước của giao dịch, không bảo đảm bắt đúng lúc một lời gọi Windows đang chờ.

```powershell
& $cli get --raw
& {
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    & $cli volume-down --unplug-test
    $timer.Stop()
    "exit=$LASTEXITCODE elapsed_ms=$($timer.ElapsedMilliseconds)"
}
```

Cắm lại DAC sau khi CLI thoát, rồi chạy hai lần đọc mới.

```powershell
& $cli get
& $cli get --raw
```

Lưu thông báo lỗi, mã thoát, tổng thời gian chạy (có 5 giây tạm dừng nếu đã tới mốc) và dòng `Time from resuming I/O to result` (không tính khoảng tạm dừng). Nếu không thấy thông báo rút, không tính lần đó là rút giữa giao dịch. Ghi lại cả hai mã và mức đọc sau kết nối lại. Nếu hai mã lệch, giữ chúng làm bằng chứng trước khi chỉnh balance lại.

## Kết quả

| Kiểm tra | Kết quả |
| --- | --- |
| Format, test, Clippy `-D warnings`, release | Qua; 7 test, gồm thứ tự và byte report, chặn ở nấc tối thiểu hoặc balance chưa xác nhận, dừng khi lỗi sau ghi trái, và từ chối xác nhận khi chỉ một thanh ghi khớp |
| Release trong sandbox | `list`: không thấy collection, exit 0. `get` và `get --raw`: không thấy control collection, exit 1 |
| Đọc trước ghi trên DAC thật | Người dùng chạy `get --raw` trong PowerShell ngoài sandbox và nhận `step=40 reg_0x10=0x46 reg_0x11=0x46`. Nếu lần đọc ngay trước ghi vẫn ở nấc 40, đích theo bảng là nấc 39 với mã `0x48 / 0x48`. |
| Một lần giảm và `get` từ tiến trình mới | `volume-down` ghi từ nấc 40 (`46 / 46`) xuống 39 (`48 / 48`), báo `Verified` và thoát mã 0. Tiến trình `get` mới in `39`. Tiến trình `get --raw` mới in `step=39 reg_0x10=0x48 reg_0x11=0x48`. CLI đã nhận gói khớp mã lệnh ở cả 10 lượt ghi theo điều kiện thành công. Chưa lưu các byte khác trong những gói này. |
| Rút giữa giao dịch, rồi cắm lại | Đọc trước thử ở nấc 38 (`4A / 4A`), đích nấc 37 (`4C / 4C`). Rút sau phản hồi lệnh `0x13` cho thanh ghi `0x10`. CLI lỗi khi gửi `0x03`, báo `Volume outcome unknown` và thoát mã 1. Sau khi cắm lại, tiến trình `get` mới in `37` và `get --raw` in `step=37 reg_0x10=0x4C reg_0x11=0x4A`. |

Lỗi Windows ở lần rút cáp là `WriteFile: (0x0000048F) The device is not connected.` CLI in `Time from resuming I/O to result: 0 ms`, tức phép đo từ lúc gửi tiếp đến lúc có kết quả dưới 1 ms theo bộ đếm của CLI. Stopwatch ngoài báo `exit=1 elapsed_ms=5494`. Số 5.494 ms gồm khoảng chờ chủ động 5 giây và thời gian giữa các lệnh được nhập ở từng dấu nhắc PowerShell. Vì vậy số này là thời gian đo của phiên thử, chưa tách riêng thời gian chạy tiến trình. Lỗi đến ngay khi gửi tiếp, trước khi chờ phản hồi. Hạn chờ 500 ms và 1.000 ms nêu trên là cấu hình. Chưa đo tình huống rút khi một lời gọi HID đang chờ.

Sau khi cắm lại, `0x10` giữ mã đích `0x4C` còn `0x11` giữ mã cũ `0x4A`. `get` hiện nấc 37 vì ánh xạ theo mã lớn hơn. Cặp mã thô cho thấy lần ghi chỉ cập nhật một phần. Nếu cặp mã vẫn lệch ở lần đọc đầu của một lệnh `volume-down` mới, CLI sẽ chặn ghi vì chưa xác nhận được balance giữa.
