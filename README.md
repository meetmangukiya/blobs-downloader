# blobs-downloader

Simple downloader script to download historical blobs. Downloaded blobs responses are stored as
[jsonlines](https://jsonlines.org/) format.

```
Usage: blobs-downloader [OPTIONS] --api-url <API_URL> --from-slot <FROM_SLOT>

Options:
      --api-url <API_URL>
  -f, --from-slot <FROM_SLOT>
  -t, --to-slot <TO_SLOT>
  -c, --concurrency <CONCURRENCY>  [default: 20]
  -h, --help                       Print help
```

