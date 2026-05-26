   Compiling velvet-ballistics-fuzz v0.1.0 (/home/lewis/src/vb-femdation/vb-qi37-4-2/fuzz)
    Finished `release` profile [optimized + debuginfo] target(s) in 10.13s
    Finished `release` profile [optimized + debuginfo] target(s) in 0.05s
     Running `target/x86_64-unknown-linux-gnu/release/expr_eval -artifact_prefix=/home/lewis/src/vb-femdation/vb-qi37-4-2/fuzz/artifacts/expr_eval/ -runs=1000 /home/lewis/src/vb-femdation/vb-qi37-4-2/fuzz/corpus/expr_eval`
INFO: Running with entropic power schedule (0xFF, 100).
INFO: Seed: 2336120926
INFO: Loaded 1 modules   (12824 inline 8-bit counters): 12824 [0x55e4b9c3b343, 0x55e4b9c3e55b), 
INFO: Loaded 1 PC tables (12824 PCs): 12824 [0x55e4b9c3e560,0x55e4b9c706e0), 
INFO:        0 files found in /home/lewis/src/vb-femdation/vb-qi37-4-2/fuzz/corpus/expr_eval
INFO: -max_len is not provided; libFuzzer will not generate inputs larger than 4096 bytes
INFO: A corpus is not provided, starting from an empty corpus
#2	INITED cov: 38 ft: 39 corp: 1/1b exec/s: 0 rss: 69Mb
#9	NEW    cov: 40 ft: 41 corp: 2/4b lim: 4 exec/s: 0 rss: 69Mb L: 3/3 MS: 2 CrossOver-InsertByte-
#10	NEW    cov: 42 ft: 43 corp: 3/7b lim: 4 exec/s: 0 rss: 69Mb L: 3/3 MS: 1 ChangeByte-
#12	NEW    cov: 44 ft: 45 corp: 4/11b lim: 4 exec/s: 0 rss: 69Mb L: 4/4 MS: 2 ShuffleBytes-CopyPart-
#18	NEW    cov: 48 ft: 49 corp: 5/12b lim: 4 exec/s: 0 rss: 69Mb L: 1/4 MS: 1 ChangeByte-
#47	NEW    cov: 49 ft: 50 corp: 6/16b lim: 4 exec/s: 0 rss: 69Mb L: 4/4 MS: 4 CopyPart-ShuffleBytes-ShuffleBytes-CopyPart-
	NEW_FUNC[1/1]: 0x55e4b9b3a090  (/home/lewis/src/vb-femdation/vb-qi37-4-2/target/x86_64-unknown-linux-gnu/release/expr_eval+0x1a7090) (BuildId: 382df0cfe080fb3607698e92d06eae808dea2189)
#48	NEW    cov: 61 ft: 62 corp: 7/20b lim: 4 exec/s: 0 rss: 69Mb L: 4/4 MS: 1 ChangeBinInt-
#103	NEW    cov: 62 ft: 63 corp: 8/22b lim: 4 exec/s: 0 rss: 69Mb L: 2/4 MS: 5 CopyPart-ShuffleBytes-ChangeBinInt-ChangeBit-EraseBytes-
#113	NEW    cov: 63 ft: 64 corp: 9/25b lim: 4 exec/s: 0 rss: 69Mb L: 3/4 MS: 5 ChangeByte-CopyPart-CopyPart-ChangeBinInt-EraseBytes-
#119	NEW    cov: 64 ft: 65 corp: 10/28b lim: 4 exec/s: 0 rss: 69Mb L: 3/4 MS: 1 EraseBytes-
#164	NEW    cov: 65 ft: 66 corp: 11/30b lim: 4 exec/s: 0 rss: 69Mb L: 2/4 MS: 5 ChangeBit-CopyPart-EraseBytes-ChangeBit-EraseBytes-
#175	NEW    cov: 67 ft: 68 corp: 12/34b lim: 4 exec/s: 0 rss: 69Mb L: 4/4 MS: 1 CMP- DE: "\001\000\000\000"-
#252	REDUCE cov: 67 ft: 68 corp: 12/33b lim: 4 exec/s: 0 rss: 69Mb L: 2/4 MS: 2 InsertByte-EraseBytes-
#263	NEW    cov: 69 ft: 70 corp: 13/37b lim: 4 exec/s: 0 rss: 69Mb L: 4/4 MS: 1 CrossOver-
#310	REDUCE cov: 69 ft: 70 corp: 13/36b lim: 4 exec/s: 0 rss: 69Mb L: 3/4 MS: 2 ChangeByte-CrossOver-
#329	NEW    cov: 70 ft: 71 corp: 14/37b lim: 4 exec/s: 0 rss: 69Mb L: 1/4 MS: 4 ShuffleBytes-EraseBytes-ChangeBit-CrossOver-
#405	REDUCE cov: 70 ft: 71 corp: 14/36b lim: 4 exec/s: 0 rss: 69Mb L: 3/4 MS: 1 EraseBytes-
#439	REDUCE cov: 70 ft: 71 corp: 14/35b lim: 4 exec/s: 0 rss: 69Mb L: 2/4 MS: 4 ShuffleBytes-CrossOver-EraseBytes-EraseBytes-
#599	REDUCE cov: 70 ft: 71 corp: 14/34b lim: 4 exec/s: 0 rss: 69Mb L: 2/4 MS: 5 CopyPart-CopyPart-CopyPart-ChangeBit-CrossOver-
#806	NEW    cov: 73 ft: 74 corp: 15/40b lim: 6 exec/s: 0 rss: 69Mb L: 6/6 MS: 2 CrossOver-CopyPart-
#823	NEW    cov: 74 ft: 75 corp: 16/45b lim: 6 exec/s: 0 rss: 69Mb L: 5/6 MS: 2 ShuffleBytes-CrossOver-
#919	NEW    cov: 76 ft: 77 corp: 17/50b lim: 6 exec/s: 0 rss: 69Mb L: 5/6 MS: 1 InsertRepeatedBytes-
#961	NEW    cov: 78 ft: 79 corp: 18/56b lim: 6 exec/s: 0 rss: 69Mb L: 6/6 MS: 2 ChangeBit-CrossOver-
#999	NEW    cov: 79 ft: 80 corp: 19/61b lim: 6 exec/s: 0 rss: 69Mb L: 5/6 MS: 3 ShuffleBytes-CrossOver-InsertRepeatedBytes-
#1000	DONE   cov: 79 ft: 80 corp: 19/61b lim: 6 exec/s: 0 rss: 69Mb
###### Recommended dictionary. ######
"\001\000\000\000" # Uses: 27
###### End of recommended dictionary. ######
Done 1000 runs in 0 second(s)

EXIT_STATUS=0
