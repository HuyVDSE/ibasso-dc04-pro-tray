# Kiểm chứng CLI gain trên DC04 Pro FPGA 1.58

Ngày thử 18/11/2026, giờ UTC+07:00. Quy trình tham chiếu ở [mục 4 của tài liệu gain](./dc04pro-v0.2.0-gain-level.md#4-bài-thử-phần-cứng-cho-fpga-158). CLI và Cargo đều ở bản `0.1.2`. Người dùng xác nhận đã cắm DAC, tháo tai nghe, đặt volume thấp và chuẩn bị đầu đo trước lần ghi đầu tiên.

## Kiểm tra trước khi ghi

- **Đạt — Windows MSVC.** `cargo test --offline --locked` chạy qua 18 test. `cargo build --offline --locked --release` thành công. Cả hai dùng `--manifest-path tray/Cargo.toml --target x86_64-pc-windows-msvc`. Test bao phủ hai report đủ 17 byte của cả ba mức, timeout ở từng report và mất hiệu lực phiên giữa hai report trên transport giả. SHA-256 của `dc04-cli.exe` là `80C770EF4658D058ACD0DD9C62157F0FC9C56715CCED3C0CC8B1016BA231006A`.
- **Đạt — Nhận diện DAC và FPGA.** Một iBasso-DC04-Pro, VID:PID `262A:187F`, usage page/usage `000C:0001`, interface `0`. PnP thấy thiết bị nhưng CLI trong sandbox không liệt kê được HID. Chạy CLI ngoài sandbox cho phép đọc thiết bị.
- **Đạt — Lệnh mở phiên ban đầu.** `--trace fpga` nhận input 32 byte có `[6]=58`, `[8]=58` (hex), tương ứng FPGA 1.58 theo [mốc 1](./dc04pro-milestone-1.md). `--trace get --raw` chỉ gửi hai truy vấn `0x41` đến `0x10/0x11`, cho nấc `22`, mã `62 / 62`. Trace của hai lệnh chỉ có `0x58` và `0x41`.
- **Còn thiếu — Nền RMS.** Người dùng xác nhận không đo được RMS trong lượt thử này. Chưa ghi nhận thông số tone, tải, đầu đo, dải đo hay sai số.

## Nhật ký từng mức

Mỗi lần chạy `set-gain` mở một handle riêng. Hai report của lần ghi dùng chung handle. `HID OUT` là bộ đệm HIDAPI, gồm ID `00` và payload 16 byte. `HID IN matched` là toàn bộ payload input 32 byte do HIDAPI trả. Log xác nhận phản hồi theo lệnh. Xác nhận mức gain cần số đo RMS độc lập.

### 1. Low lần đầu

**Đạt HID và volume.** Ghi đủ 17 byte hai lần, nhận đúng `0x15 → 0x16`, volume trước/sau `22 (62 / 62)`. **RMS trái/phải** không đo được.

```text
2026-11-18 11:58:22 +07:00
step=22 reg_0x10=0x62 reg_0x11=0x62
HID OUT (17 bytes): [00, 15, 11, 88, 60, 00, 00, 05, 08, 00, 00, 00, 00, 00, 00, 00, 00]
HID write returned 17 bytes
HID IN matched (32 bytes): [00, 00, 00, 00, 00, 00, 15, 00, 62, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00]
Received matching reply for gain command 0x15 (DAC 0x60).
HID OUT (17 bytes): [00, 16, 11, 88, 62, 00, 00, 05, 08, 00, 00, 00, 00, 00, 00, 00, 00]
HID write returned 17 bytes
HID IN matched (32 bytes): [00, 00, 00, 00, 00, 00, 16, 00, 62, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00]
Received matching reply for gain command 0x16 (DAC 0x62).
Sent gain level: low
set-gain exit=0
step=22 reg_0x10=0x62 reg_0x11=0x62
```

### 2. Mid

**Đạt HID và volume.** Ghi đủ 17 byte hai lần, nhận đúng `0x15 → 0x16`. Volume trước/sau `22 (62 / 62)`. **RMS trái/phải** không đo được.

```text
2026-11-18 11:59:56 +07:00
Before: step=22 reg_0x10=0x62 reg_0x11=0x62
HID OUT (17 bytes): [00, 15, 11, 88, 60, 00, 00, 05, 08, 00, 00, 00, 20, 00, 00, 00, 00]
HID write returned 17 bytes
HID IN matched (32 bytes): [00, 00, 00, 00, 00, 00, 15, 00, 62, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00]
Received matching reply for gain command 0x15 (DAC 0x60).
HID OUT (17 bytes): [00, 16, 11, 88, 62, 00, 00, 05, 08, 00, 00, 00, 20, 00, 00, 00, 00]
HID write returned 17 bytes
HID IN matched (32 bytes): [00, 00, 00, 00, 00, 00, 16, 00, 62, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00]
Received matching reply for gain command 0x16 (DAC 0x62).
Sent gain level: mid
set-gain exit=0
After: step=22 reg_0x10=0x62 reg_0x11=0x62
```

### 3. High

**Đạt HID và volume.** Ghi đủ 17 byte hai lần, nhận đúng `0x15 → 0x16`. Volume trước/sau `22 (62 / 62)`. **RMS trái/phải** không đo được.

```text
2026-11-18 11:59:57 +07:00
Before: step=22 reg_0x10=0x62 reg_0x11=0x62
HID OUT (17 bytes): [00, 15, 11, 88, 60, 00, 00, 05, 08, 00, 00, 00, 31, 00, 00, 00, 00]
HID write returned 17 bytes
HID IN matched (32 bytes): [00, 00, 00, 00, 00, 00, 15, 00, 62, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00]
Received matching reply for gain command 0x15 (DAC 0x60).
HID OUT (17 bytes): [00, 16, 11, 88, 62, 00, 00, 05, 08, 00, 00, 00, 31, 00, 00, 00, 00]
HID write returned 17 bytes
HID IN matched (32 bytes): [00, 00, 00, 00, 00, 00, 16, 00, 62, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00]
Received matching reply for gain command 0x16 (DAC 0x62).
Sent gain level: high
set-gain exit=0
After: step=22 reg_0x10=0x62 reg_0x11=0x62
```

### 4. Low lần hai

**Đạt HID và volume.** Ghi đủ 17 byte hai lần, nhận đúng `0x15 → 0x16`. Volume trước/sau `22 (62 / 62)`. **RMS trái/phải** không đo được.

```text
2026-11-18 11:59:57 +07:00
Before: step=22 reg_0x10=0x62 reg_0x11=0x62
HID OUT (17 bytes): [00, 15, 11, 88, 60, 00, 00, 05, 08, 00, 00, 00, 00, 00, 00, 00, 00]
HID write returned 17 bytes
HID IN matched (32 bytes): [00, 00, 00, 00, 00, 00, 15, 00, 62, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00]
Received matching reply for gain command 0x15 (DAC 0x60).
HID OUT (17 bytes): [00, 16, 11, 88, 62, 00, 00, 05, 08, 00, 00, 00, 00, 00, 00, 00, 00]
HID write returned 17 bytes
HID IN matched (32 bytes): [00, 00, 00, 00, 00, 00, 16, 00, 62, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00]
Received matching reply for gain command 0x16 (DAC 0x62).
Sent gain level: low
set-gain exit=0
After: step=22 reg_0x10=0x62 reg_0x11=0x62
```

## Mở lại CLI và rút/cắm USB

### Mở lại CLI

**Đạt trên CLI.** Sau Low lần hai, các tiến trình mới liệt kê được DAC, đọc được FPGA 1.58 và volume `22 (62 / 62)`. Trace chỉ có truy vấn `0x58` và hai truy vấn `0x41`. **RMS trái/phải** không đo được.

```text
2026-11-18 12:00:07 +07:00
iBasso-DC04-Pro VID=262a PID=187f usage_page=000c usage=0001 interface=0 path=\\?\HID#VID_262A&PID_187F&MI_00#8&26d80aa6&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}
HID OUT (17 bytes): [00, 58, 12, DC, 2C, 04, 00, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00]
HID write returned 17 bytes
HID IN matched (32 bytes): [00, 00, 00, 00, 00, 00, 58, 01, 58, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00]
FPGA version byte: 0x58
HID OUT (17 bytes): [00, 41, 12, E4, A2, 00, 10, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00]
HID write returned 17 bytes
HID IN matched (32 bytes): [00, 00, 00, 00, 00, 00, 41, 01, 62, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00]
HID OUT (17 bytes): [00, 41, 12, E4, A2, 00, 11, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00]
HID write returned 17 bytes
HID IN matched (32 bytes): [00, 00, 00, 00, 00, 00, 41, 01, 62, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00]
step=22 reg_0x10=0x62 reg_0x11=0x62
```

### Sau khi rút USB

**Đạt.** Người dùng xác nhận để DAC rời máy. PnP và CLI đều không thấy DAC. Truy vấn volume trả exit `1` tại bước mở collection. Trace không có report nào.

```text
2026-11-18 12:03:59 +07:00
Microsoft PnP Utility

No devices were found on the system.
No iBasso DC04 Pro HID collections found.
list exit=0
No DC04 Pro control collection (262a:187f, 000c:0001) found
get exit=1
```

### Sau khi cắm lại USB

**Đạt trên CLI.** CLI nhận lại collection `262A:187F / 000C:0001`, đọc FPGA 1.58 và volume `22 (62 / 62)`. Trace chỉ có `0x58` và hai lệnh `0x41`. **RMS trái/phải** không đo được.

```text
2026-11-18 12:04:20 +07:00
iBasso-DC04-Pro VID=262a PID=187f usage_page=000c usage=0001 interface=0 path=\\?\HID#VID_262A&PID_187F&MI_00#8&26d80aa6&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}
HID OUT (17 bytes): [00, 58, 12, DC, 2C, 04, 00, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00]
HID write returned 17 bytes
HID IN matched (32 bytes): [00, 00, 00, 00, 00, 00, 58, 01, 58, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00]
FPGA version byte: 0x58
HID OUT (17 bytes): [00, 41, 12, E4, A2, 00, 10, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00]
HID write returned 17 bytes
HID IN matched (32 bytes): [00, 00, 00, 00, 00, 00, 41, 01, 62, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00]
HID OUT (17 bytes): [00, 41, 12, E4, A2, 00, 11, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00]
HID write returned 17 bytes
HID IN matched (32 bytes): [00, 00, 00, 00, 00, 00, 41, 01, 62, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00]
step=22 reg_0x10=0x62 reg_0x11=0x62
```

## Tổng hợp tiêu chí

| Tiêu chí | Trạng thái |
| --- | --- |
| Low → Mid → High → Low có hai lần ghi đủ 17 byte và hai phản hồi khớp theo đúng thứ tự ở mỗi mức | Đạt trên DAC |
| Volume giữ nguyên ở từng mức | Đạt trên DAC. Nấc 22, mã `62 / 62` trước/sau cả bốn lần |
| RMS hai kênh tăng Low → Mid → High vượt sai số, High không clipping | Chưa có số đo |
| Hai kênh trở lại mốc Low, tái lập Mid và High | Đã gửi Low lần hai và có hai phản hồi. Còn thiếu RMS và lượt lặp Mid/High |
| CLI không ghi gain tự phát khi mở lại | Đạt. Trace chỉ có lệnh truy vấn |
| CLI báo mất kết nối khi rút DAC | Đạt. `get --raw` trả exit `1` tại bước mở collection |
| Nhận diện lại DAC sau khi cắm lại và không phát lại gain | Đạt. CLI đọc lại được FPGA và volume, trace chỉ có lệnh truy vấn |
| Gain thực tế sau khi mở lại CLI hoặc sau khi rút/cắm | Chưa có RMS hoặc readback gain để xác định |
| Rút DAC đúng giữa hai report | Chưa thử trên DAC. Hai nhánh timeout và dừng giữa hai report đã qua test transport giả |

## Quyết định phát hành v0.2.0

Người dùng xác nhận đã nghiệm thu bước kiểm tra rút cáp và tray, đồng thời yêu cầu phát hành vì chưa có thiết bị đo RMS. Bảng trên ghi riêng bằng chứng từ đợt thử CLI, gồm các phản hồi HID và volume trước/sau. Chưa có số đo độc lập để xác định tác dụng của từng mức trên hai kênh hoặc mức còn giữ sau khi mất nguồn USB. Cửa sổ DAC Settings tiếp tục hiển thị `Current gain: Unknown` và chỉ hiện `Last sent` sau khi nhận đủ hai phản hồi khớp.
