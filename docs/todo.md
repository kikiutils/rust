# TODO

- [ ] logger 加回內建 file rotation，依檔案大小觸發：
  - 在單執行緒的 `FileSink` worker 裡做，不需要鎖：`FileSink` 為每個開啟的檔案累計已寫入的 bytes，超過門檻就 flush、關閉、rename、重新開檔。開檔時從 metadata 讀出既有大小當累計的起點，否則重啟後會從 0 重算。
  - 備份檔名為 `{stem}.<timestamp>.log`，時間戳精確到微秒，rename 前還要檢查目標是否已存在。Windows 的 `fs::rename` 會直接覆蓋既有檔案。
  - 每個 key 最多保留 `max_backups` 份，超過就從最舊的刪起。只刪符合 `{stem}.<timestamp>.log` 格式的檔案，不能誤刪其他 target 的檔。只 rename 不刪舊檔，硬碟一樣會被寫滿。
  - rename 失敗時繼續寫原檔，並把下次檢查的門檻往後延，避免每寫一行就重試一次。
  - `_fallback.log` 也要一起 rotation。
  - 介面：`LoggerFileOutputOptions::with_max_file_bytes(Option<u64>)`（`None` 表示不 rotation）、`with_max_backups(usize)`；環境變數 `LOGGER_FILE_MAX_BYTES`、`LOGGER_FILE_MAX_BACKUPS`。
  - 假設不會有外部 logrotate，所以不提供 `LoggerGuard::reopen()` 或 SIGHUP。
