# Gain Level cho iBasso DC04 Pro Tray v0.2.0

Phân tích tĩnh ngày 18/11/2026, trên nền tray v0.1.2. Bản web cho biết chính xác cách **ghi** Low, Mid, High. Khi mở kết nối, web hiển thị **Low mặc định**. Chưa có phép đọc gain đã kiểm chứng cho FPGA 1.58. Đề xuất tray hiển thị `Current gain: Unknown` và chỉ ghi khi người dùng chọn mức. Bài thử ở mục 4 dành cho task phần cứng tiếp theo. [Roadmap](../docs/roadmap.md) · [Thiết kế hệ thống](../docs/system-design.md)

## 1. Từ nút bấm đến hai HID Output Report

Trong `Dc04ProParamsCard`, nút phát `g("setGain",u)` với `u` là chỉ số trong danh sách. `Dc04ProControlsPanel` dựng danh sách `j` theo thứ tự low, mid, high và nối `onSetGain:Q` tới `Q(t){c.value?.setGain(t)}`. Ở đây `c` trỏ tới controller của thiết bị đang kết nối. [View dành riêng cho DC04 Pro](../local-web/assets/Dc04ProView-CeT831J1.js)

Trong factory `yd`, nhánh `case de.DC04PRO:return new _n(e)` chọn `class _n extends Pt`; nhánh DC04U dùng `Jt`. DC04 Pro kế thừa `Pt.setGain(t)`. Hàm lấy `r = Pt.GAIN_VALUES[t] ?? 0` từ bảng `[0,32,49]`, lần lượt chờ `writeReg(21,96,8,0,0,0,r)` và `writeReg(22,98,8,0,0,0,r)`, rồi mới gọi `updateDeviceParams({gain:t})`. [Factory, controller và transport](../local-web/assets/index-DLTNBGaH.js)

| Lựa chọn | Chỉ số UI | Giá trị gửi (thập phân / hex) | Report thứ nhất (lệnh, địa chỉ, thanh ghi, giá trị), hex | Report thứ hai, cùng thứ tự trường |
| --- | ---: | --- | --- | --- |
| Low | 0 | 0 / 00 | 15 60 08 00 | 16 62 08 00 |
| Mid | 1 | 32 / 20 | 15 60 08 20 | 16 62 08 20 |
| High | 2 | 49 / 31 | 15 60 08 31 | 16 62 08 31 |

`Pt.writeReg → Pt.buildReg → pn.sendOutputReportWithResponse → It.sendOutputReportWithCmdResponse → device.sendReport(0, payload)`. Hai payload đầy đủ bên dưới dùng hex, với `GG` lấy từ bảng. `buildReg` đặt giá trị ở offset 11, tính từ 0. WebHID truyền ID 0 riêng. HIDAPI Windows cần thêm byte ID `00` phía trước mỗi payload, tổng cộng 17 byte cho mỗi lần gọi `write`. [Bundle web](../local-web/assets/index-DLTNBGaH.js) · [Descriptor và quy ước HIDAPI đã kiểm tra](./dc04pro-milestone-1.md)

```text
15 11 88 60 00 00 05 08 00 00 00 GG 00 00 00 00
16 11 88 62 00 00 05 08 00 00 00 GG 00 00 00 00
```

Transport đăng ký callback theo mã lệnh, mặc định chờ 500 ms. Với input ID 0 dài 32 byte, `It.handleInputReport` dùng byte payload [6] để ghép phản hồi. Do hai lần `await` tuần tự, lỗi lần đầu chặn lần hai. Lỗi lần hai có thể để hai địa chỉ ở hai mức khác nhau. `setGain` bỏ qua nội dung phản hồi sau khi ghép, không truy vấn lại thanh ghi. [Bundle web](../local-web/assets/index-DLTNBGaH.js)

## 2. Nguồn của mức gain khi vừa kết nối

1. Store `W.connect` xóa `deviceParams`, mở thiết bị, chọn controller bằng `yd` rồi chờ `initialize()`. `resetConnectionState` cũng xóa `deviceParams`.
2. `_n.initialize` gọi `super.initialize()`. Chính `Pt.initialize` gán `gain:0` vào store trong bộ nhớ. Sau đó `_n` gán `ecoMode:0` và gọi `getFpgaVersion()` bằng lệnh `0x58`. Đường khởi tạo này không gọi `setGain(0)`, truy vấn gain hay khôi phục gain đã lưu. `Pt.parseResponseData` chỉ gọi `printLog`. [Store và controller](../local-web/assets/index-DLTNBGaH.js)
3. View lấy `a = computed(() => g.dc04u)`; getter tên `dc04u` ở đây trả về `deviceParams`. Biểu thức `J = computed(() => a.value.gain ?? 0)` còn có mặc định 0 khi thiếu trường. Nút sáng theo `d.gainValue === u`. Sau một lần chọn thành công, giao diện hiển thị chỉ số vừa gửi trong phiên. Kết nối mới lại bắt đầu ở Low dù phiên trước chọn High. [View DC04 Pro](../local-web/assets/Dc04ProView-CeT831J1.js)

Phần còn thiếu là truy vấn đọc gain và cách giải mã phản hồi trên cả hai địa chỉ, cùng hành vi thiết bị khi mất nguồn. Cờ `_n.CAPS.persistsSettings` bằng false chỉ mô tả capability trong web. Ghi chú trước đây xác nhận web nhận diện FPGA 1.58 và có ba nút gain. Các thử nghiệm đọc/ghi có đối chứng trong những mốc sau là cho **volume**, bao gồm truy vấn `0x41` tới hai thanh ghi volume `0x10/0x11`. Chưa có căn cứ dùng truy vấn đó làm readback gain. [Ghi chú thiết bị](./dc04pro-software-research.md) · [Mốc 1](./dc04pro-milestone-1.md) · [Mốc 2](./dc04pro-milestone-2.md) · [Đối chiếu trạng thái web](./dc04pro-milestone-4.md)

## 3. Thay đổi tối thiểu trong Rust

| Thành phần | Phần dùng lại và thay đổi đề xuất |
| --- | --- |
| Protocol | Thêm enum `GainLevel::{Low, Mid, High}` và `gain_writes(level)` trả đúng hai report. Hàm `dac_write(command,address,register,channel,code)` đã tạo đúng bố cục nếu truyền register `0x08`, channel `0`. Test cả 17 byte và thứ tự cho ba mức. [protocol.rs](../tray/src/dc04/protocol.rs) |
| HID và Device | Dùng `Device::open` chọn duy nhất collection VID:PID `262A:187F`, usage page/usage `000C:0001`. Tổng quát hóa `hid::write_volume` để dùng chung `exchange` với tên thao tác phù hợp. Hàm này đã kiểm tra độ dài ghi, timeout và dùng `matching_response` để bỏ report nút `FE 01`, ghép lệnh [6]. Thêm `Device::set_gain(level, active)` kiểm tra phiên trước mỗi report, dừng ngay khi lỗi, ghi rõ lệnh nào đã có phản hồi và lệnh nào lỗi. Kết quả thành công chỉ có nghĩa đã nhận đủ hai phản hồi khớp. [mod.rs](../tray/src/dc04/mod.rs) · [hid.rs](../tray/src/dc04/hid.rs) |
| Worker | Giữ một handle trên thread `dc04-hid`, xử lý trọn cặp report của một yêu cầu trước giao dịch tiếp. Thêm `Work::WriteGain`, kết quả và trường snapshot gain riêng. `State::confirm(VolumeReading)`, `pending/requested` và `Snapshot::adjustable` hiện phục vụ slider. Chỉ cho chọn gain khi có một thiết bị sẵn sàng trong epoch hiện tại và chưa có lần ghi gain đang chờ. Kiểm tra epoch cả lúc nhận yêu cầu lẫn lúc công bố kết quả. Dùng `State::invalidate` để hủy yêu cầu gain khi đổi phiên. Giữ chẩn đoán lỗi gain đủ lâu để đọc vì lần đọc volume thành công hiện xóa `last_error`. [worker.rs](../tray/src/worker.rs) · [Mốc 3](./dc04pro-milestone-3.md) |
| Tray | Thêm một lệnh `DAC Settings` trong `Tray::menu` mở cửa sổ Win32 nhỏ với nhóm `Gain Level` và ba nút Low/Mid/High. Giữ lối bấm trái vào popup volume. Dùng `PostMessageW(SNAPSHOT)` và `App::render` để cập nhật cửa sổ từ snapshot, dùng đường `WM_DEVICECHANGE` hiện có để rescan. Mở cửa sổ chỉ hiển thị trạng thái. [tray.rs](../tray/src/win32/tray.rs) · [win32/mod.rs](../tray/src/win32/mod.rs) |

| Tình huống | Cửa sổ DAC Settings |
| --- | --- |
| Phiên mới, đã sẵn sàng | Hiện `Current gain: Unknown`, không đánh dấu nút nào. Cho phép người dùng chủ động chọn mức. |
| Người dùng chọn Mid | Hiện `Sending Mid…`, tạm khóa lựa chọn. Khi cả hai phản hồi khớp, hiện `Last sent: Mid` và vẫn ghi `Current gain: Unknown`. Chỉ hiển thị mức hiện tại khi có readback đã kiểm chứng trong phiên. |
| Gửi lỗi, ghi ngắn hoặc timeout | Báo `Gain write failed — current gain unknown`, kèm lệnh/địa chỉ và lý do. Nếu lần hai lỗi, nêu lần đầu đã có phản hồi và hai kênh có thể lệch. Bỏ đích chờ, khóa khi worker phục hồi kết nối, mở handle mới theo đường lỗi hiện có. Lần ghi tiếp cần thao tác chọn mới. Không tự ghi bù hay hoàn tác. |
| Rút/cắm hoặc rescan | Khi rút, vô hiệu hóa epoch, xóa lựa chọn vừa gửi và yêu cầu chờ, bỏ kết quả đến muộn, khóa các nút. Khi cắm lại, đọc volume theo luồng hiện có và đưa gain về Unknown. Mất kết nối giữa hai report cũng đi theo luồng này. |

## 4. Bài thử phần cứng cho FPGA 1.58

**Chuẩn bị.** Chỉ cắm một DC04 Pro, ghi lại VID/PID, collection và FPGA đọc bằng CLI. Tháo tai nghe, dùng tải giả phù hợp và đầu đo RMS hai kênh hoặc audio interface có đủ headroom, tắt AGC. Phát tone cố định, chẳng hạn 1 kHz ở −30 dBFS. Chọn volume DAC thấp nhưng đủ đo, ghi mã volume trái/phải bằng CLI. Giữ nguyên volume nguồn, volume DAC, balance, output mode, eco, cổng ra, tải và dải đo suốt loạt thử. Ghi điều kiện chưa biết thay vì lấy mặc định trên web. Chỉ một trình điều khiển được ghi tại một thời điểm.

**Nhật ký.** Với mỗi bước, lưu thời điểm, phiên, mức yêu cầu, hai payload gửi, số byte HIDAPI thực ghi, input hex tương ứng hoặc lỗi/timeout, volume trước/sau và điện áp RMS trái/phải khi tín hiệu ổn định. Bảng dưới là phiếu ghi cho task sau, các ô quan sát mô tả số liệu cần thu.

| Trạng thái ban đầu | Thao tác | Quan sát cần ghi | Tiêu chí đạt |
| --- | --- | --- | --- |
| Cắm mới, gain chưa biết | Mở phiên, đo nền và RMS ban đầu, chưa gửi lệnh gain | FPGA, volume, RMS L/R, log mở kết nối | Đúng FPGA 1.58, có số đo ổn định ở cả hai kênh, log không có lệnh gain tự phát. |
| Gain chưa biết | Chọn Low | Cặp lệnh với GG=00, RMS Low lần 1 | Đủ hai phản hồi đúng thứ tự. Lấy mốc Low, xác minh lại bằng bước High → Low bên dưới vì trạng thái ban đầu có thể đã là Low. |
| Low vừa đo | Chọn Mid | Cặp lệnh GG=20, RMS Mid | Đủ hai phản hồi, RMS tăng so với Low ở cả hai kênh vượt sai số đo. |
| Mid vừa đo | Chọn High | Cặp lệnh GG=31, RMS High | Đủ hai phản hồi, RMS tăng so với Mid ở cả hai kênh vượt sai số đo, tín hiệu chưa clipping. |
| High vừa đo | Chọn Low rồi lặp lại Mid và High | Log từng cặp, RMS lặp lại, volume L/R | Cả hai kênh về mốc Low và tái lập ba mức trong sai số. Mã volume trước/sau giống nhau. |
| High đã đo | Đóng/mở công cụ rồi rút/cắm, đo trước khi ghi gain ở mỗi phiên mới | Có thể cần phát lại cùng tone. Ghi volume, RMS và mọi lệnh trong phiên | App bắt đầu Unknown, không phát lại đích cũ. Chỉ kết luận gain còn giữ hoặc đã đổi nếu điều kiện đo tương đương và số đo khớp mốc. Ghi riêng kết quả mở handle mới và mất nguồn USB. |
| Low đã đo, chuẩn bị đổi Mid | Trong CLI chẩn đoán, cho dừng giữa hai report sau phản hồi 15, rút cáp rồi mới cho tiếp tục. Thử thêm timeout/lỗi qua transport giả để tái lập nhánh lỗi. | Report cuối đã gửi, lỗi, epoch cũ/mới, RMS L/R sau khi cắm lại nếu đo được | UI/CLI báo kết quả chưa biết hoặc mất kết nối. Phiên cũ không gửi report 16 sang phiên mới. Không tự lặp lệnh. Sau phép đo, chủ động chọn Low và kiểm tra lại hai kênh. |

**Cách kết luận.** Bằng chứng giao dịch gồm `write == 17` cho mỗi report và input 32 byte có [6] lần lượt bằng `15`, `16` (hex). Ghi đủ byte chứng minh backend chấp nhận report; phản hồi khớp chứng minh có trả lời từng lệnh. Để xác nhận **tác dụng của gain**, so sánh RMS theo từng kênh ở cùng điều kiện, ghi độ dao động qua các lần đo và sai số của dụng cụ. Yêu cầu ba mức phân biệt được, lặp lại theo thứ tự Low < Mid < High; có thể báo chênh lệch bằng `20 log10(V2/V1)` dB. Chưa có chênh lệch dB danh định đã xác minh từ nguồn này. Nếu chỉ một kênh đổi, số đo chồng lấn, clipping hoặc volume thay đổi, ghi kết quả chưa đạt/chưa đủ bằng chứng. Nếu sau này tìm được readback gain độc lập, đối chiếu giá trị đọc mới của cả hai địa chỉ với từng mức và với số đo. Ảnh nút đang sáng hay cảm giác nghe to hơn không đủ để nghiệm thu.

## Kết luận của phân tích ban đầu

- **Đã xác minh từ mã** — Trên DC04 Pro, Low/Mid/High dùng mã `00/20/31` hex, mỗi lần ghi thanh ghi `08` ở hai địa chỉ theo thứ tự `60 → 62`. Low lúc kết nối xuất phát từ store/UI. REA phân tích tĩnh hai bundle, evidence `ev_84642ad5596ebe012234a92e62ff241e4bc7fa1574d3f64d1645aefb1c7414f3`. Các tên hàm ở trên thuộc bản lưu có SHA-256 `687C7107FFB68B95385661D3928C3833BD36110855212B6E3774D9D2F18031EE` (bundle chính) và `E86A4ADB62E6329EB41C34E0662A35D921110908BFF071906D941F085E63B03C` (view).
- **Cần thử trên phần cứng** — Thu phản hồi thật, xác nhận cả hai kênh đổi theo đủ ba mức, đo sau khi mở lại và sau mất nguồn, kiểm tra tình huống ghi dở. Chưa xác lập được lệnh đọc gain từ mã DC04 Pro đã xét.
- **Task triển khai nhỏ nhất** — Thêm enum và bộ dựng hai report có test byte, dùng chung transport cho một lệnh CLI chẩn đoán `set-gain low|mid|high` có log từng bước. Dùng CLI cho bài thử FPGA 1.58 ở trên. Sau khi đạt tiêu chí, mới nối lệnh đã kiểm chứng vào worker và cửa sổ DAC Settings.

CLI, worker và DAC Settings đã được triển khai cho v0.2.0. Xem [kết quả kiểm thử và quyết định phát hành](./dc04pro-v0.2.0-gain-validation.md) để biết phần đã kiểm chứng và phần còn thiếu số đo.
