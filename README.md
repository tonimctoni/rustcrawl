# rustcrawl

Yet another webcrawler. This time with cool ideas as to how to avoid using very large hashtables.

I looked it up, it is called a bloom filter.

Also, the reservoir of urls to be crawled is finite. If full, inserts remove random previous elements, and crawler crawls less urls.

# notes for myself

 - Storing strings instead of urls makes it more space efficient, but less time efficient (worth it).

 - If all threads use tha same seed (which I dont know if they would) that is not a problem, as long as statistical properties for each individual thread are ok.

 - Relaxed ordering for counters should be enought. Maybe ask someone who knows though. Then again, who knows.

# todo

 - Add sleep at the beginning of each thread's loop.

 - If reservoir is running out of space, the urls per crawl should be reduced.
    - Decided against it, actually. Revise later.

<!--  - Already used urls might get added to the reservoir, see if it is worth it to change that (use unique_gathered, unique_visited).
    - Actually, just not add urls to the reservoir that have been crawled already. Urls in reservoir might get discarded after all. -->

 <!-- - Add a check/set in one method to unique. -->

<!--  - Add logger struct that logs errors.
    - Actually, maybe just print to stdout (or stderr). -->

 - Make sure to use large unique when deploying.

 <!-- - Use atomic counter to count gathered and visited pages. Maybe log this every so often. Also count css files. -->

 - Document everything (as in, write comments and use rustdoc).

 - Use box syntax in unique module.

 - Test further by using.