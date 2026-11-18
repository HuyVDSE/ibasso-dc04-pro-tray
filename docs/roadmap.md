# Roadmap iBasso DC04 Pro Tray

Mốc xuất phát là v0.1.2. App đã có popup chỉnh volume phần cứng từ 1–99, tự nhận lại DAC khi cắm lại và có tùy chọn chạy cùng Windows. Mỗi mốc tính năng mới tăng số giữa và bắt đầu ở v0.x.0. Các bản v0.x.y với y ≥ 1 dành cho sửa lỗi của mốc đó. Ví dụ, v0.2.1 sửa lỗi cho v0.2.0. Ưu tiên đề xuất là đưa năm điều khiển của bản web vào tray trước, sau đó thêm các tiện ích riêng cho Windows.

| Mốc | Người dùng làm được gì? | Điều kiện để phát hành |
| --- | --- | --- |
| v0.2.0–v0.6.0 | Chỉnh năm thông số DAC từ tray | Phát hành riêng từng điều khiển sau khi kiểm chứng trên DC04 Pro thật |
| v0.7.0 | Chỉnh volume từ ứng dụng đang dùng và thấy mức DAC vừa xác nhận | Phím tắt và lớp hiển thị hoạt động khi popup đóng, kể cả sau khi cắm lại DAC |
| v0.8.0 | Chọn tai nghe trong menu để áp dụng volume và mức trần đã lưu | Mọi đường chỉnh volume trong app đều tuân theo trần đang chọn |
| v0.9.0 | Thấy mức volume khi bấm nút trên thân DAC | Đọc đúng thay đổi khi popup đóng với chi phí chạy nền chấp nhận được |

## v0.2.0–v0.6.0 · Cài đặt DAC như bản web

Đặt các điều khiển DAC ngay dưới thanh volume trong popup mở bằng chuột trái. Mục **DAC Settings** trong menu chuột phải mở cùng popup. Phạm vi điều khiển dựa trên giao diện web DC04 PRO trong ảnh người dùng cung cấp.

| Bản dự kiến | Điều khiển | Các lựa chọn trên web |
| --- | --- | --- |
| v0.2.0 | Gain Level | Low, Mid, High |
| v0.3.0 | Output Mode | Normal, Boost |
| v0.4.0 | Eco Mode | Off, On |
| v0.5.0 | Digital Filter | Fast Roll-off, Slow Roll-off, LD Fast Roll-off, LD Slow Roll-off, NOS |
| v0.6.0 | Channel Balance | L10 đến R10, điểm giữa 0-0 |

1. Đối chiếu từng lựa chọn với lệnh trong bản web đã lưu. Dùng CLI kiểm tra trên DC04 Pro FPGA 1.58 để xác định cách ghi, khả năng đọc lại và trạng thái sau khi rút/cắm.
2. Thử Gain Level và Output Mode ở volume thấp, tháo tai nghe khi thử chuyển chế độ. Kiểm tra việc bật/tắt Eco Mode có ảnh hưởng tới gain hoặc output hay không để giao diện phản ánh đúng hành vi đã đo.
3. Mỗi bản chỉ cần nghiệm thu đủ các lựa chọn của điều khiển tương ứng và kiểm tra lại các điều khiển đã phát hành. Với thuộc tính đọc lại được, hiển thị giá trị DAC trả về. Nếu chỉ xác minh được thao tác ghi, phân biệt lựa chọn vừa gửi với trạng thái thực của DAC và hiển thị “Unknown” khi mở một phiên kết nối mới.

### Kiểm thử riêng cho Channel Balance

1. Bắt đầu bằng công cụ CLI để đối chiếu mức giữa, lệch trái và lệch phải với bộ điều khiển của hãng. Kiểm tra ở nhiều mức volume vì cùng một cặp giá trị đọc về có thể chưa đủ để xác định balance.
2. Thử rút cáp giữa lần ghi hai kênh. Chỉ mở chỉnh balance trong tray khi app nhận biết được trạng thái chưa xác nhận và có đường phục hồi đã kiểm chứng.
3. Nghiệm thu bằng cách chỉnh balance, đổi volume rồi đọc lại cả hai kênh. Khi gặp lỗi giữa lần ghi, app phải báo cần đồng bộ lại và chờ kết quả đọc trước thao tác tiếp theo.

## v0.7.0 · Phím tắt và hiển thị âm lượng

1. Thêm hai phím tắt toàn hệ thống để tăng và giảm một nấc qua worker hiện có. Cho phép đổi tổ hợp, tắt phím tắt và lưu lựa chọn qua lần khởi động sau.
2. Khi DAC xác nhận mức mới, hiện ngắn gọn `DC04 Pro · 24/99` ở góc màn hình. Lớp hiển thị không lấy focus. Nếu tổ hợp đã bị ứng dụng khác giữ, báo rõ tổ hợp bị trùng.
3. Nghiệm thu trên Windows 11 với popup đóng. Thử bấm nhanh nhiều lần, chạm hai đầu dải, rút DAC khi đang chỉnh rồi cắm lại. Đối chiếu số hiện trên màn hình với mức đọc lại từ DAC. Lệnh từ trước khi rút phải được hủy.

## v0.8.0 · Preset theo tai nghe và mức trần

1. Cho phép lưu volume hiện tại thành preset có tên, đặt mức trần và chọn preset từ menu chuột phải. Nếu mức lưu vượt trần, giao diện yêu cầu sửa trước khi lưu.
2. Dùng cùng một bước kiểm tra trần cho slider, con lăn, phím tắt và preset. Chỉ áp volume của preset khi người dùng chủ động chọn. Khi khởi động hoặc cắm lại DAC, đọc mức thực tế trước.
3. Nghiệm thu bằng hai preset có mức trần khác nhau. Thử mọi cách tăng volume, chuyển preset, khởi động lại và cắm lại DAC. Nếu nút vật lý đưa volume vượt trần, app hiển thị mức thực và cho phép hạ về trần.

## v0.9.0 · Hiển thị thay đổi từ nút vật lý

1. Thử trên DAC thật xem bấm nút có tạo sự kiện HID mà app nhận được không. Nếu phải đọc định kỳ, đo độ trễ hiển thị, CPU và số lần đọc lúc rảnh trước khi chọn chu kỳ.
2. Dùng lại lớp hiển thị của v0.7.0. Phân biệt thay đổi từ nút vật lý với lần ghi vừa được app xác nhận để mỗi thao tác chỉ hiện một lần.
3. Nghiệm thu khi popup đóng, khi giữ nút vật lý và sau khi máy sleep/wake. Khi rút DAC, app dừng đọc thiết bị cũ. Nếu đọc nền gây tải hoặc xung đột với thao tác chỉnh, giữ tính năng ở giai đoạn thử nghiệm.

## Việc bắt đầu ngay

Bắt đầu với Gain Level. Xác định lệnh cho Low, Mid và High, thử ở volume thấp với tai nghe đã tháo, rồi đưa lựa chọn gain vào DAC Settings sau khi xác minh cách báo trạng thái.

Xem thêm [thiết kế tray](system-design.md), [kết quả thử ghi hai kênh](../research/dc04pro-milestone-2.md) và [kiểm thử bản portable](../research/dc04pro-milestone-4.md).
