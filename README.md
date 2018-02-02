# rustcrawl

Yet another webcrawler. This time with cool ideas as to how to avoid using very large hashtables.

I looked it up, it is called a bloom filter.

Also, the reservoir of urls to be crawled is finite. If full, inserts remove random previous elements, and crawler crawls less urls.

# notes for myself

 - Storing strings instead of urls makes it more space efficient, but less time efficient (worth it).

 <!-- - If all threads use the same seed (which I dont know if they would) that is not a problem, as long as statistical properties for each individual thread are ok. -->

 - Relaxed ordering for counters should be enought. Maybe ask someone who knows though. Then again, who knows.

 - Using modulo to get random values within a range because rust's implementation of ind_sample seems needlessly complicated to me.

# todo

 - Add sleep at the beginning of each thread's loop.

 - If reservoir is running out of space, the urls per crawl should be reduced.
    - Decided against it, actually. Revise later.

 - Already used urls might get added to the reservoir, see if it is worth it to change that (use unique_gathered, unique_visited).
    - Actually, just not add urls to the reservoir that have been crawled already. Urls in reservoir might get discarded after all.
    - Actually actually, getting too many "url has been used" "errors", maybe use another unique.
    - Then again, a fast filling unique is not desired.

 <!-- - Add a check/set in one method to unique. -->

<!--  - Add logger struct that logs errors.
    - Actually, maybe just print to stdout (or stderr). -->

 - Make sure to use large unique when deploying.

 <!-- - Use atomic counter to count gathered and visited pages. Maybe log this every so often. Also count css files. -->

 - Document everything (as in, write comments and use rustdoc).
    - Also comment within functions.

 - Use box syntax in unique module.

 - Test further by using.

 - Maybe use hyper, reqwest feels clunky.

 - Make sure stuff works with only one thread too. So far so good.

 - Remove #![allow(dead_code)].

 - Maybe use buffer_unordered.

 - Maybe send uri to html_worker instead of string, if a move over channel is possible.

 - Maybe use regex over Vec<u8> instead of string too (or not, its a move transform).

 - Maybe a direct transformation uri->url is possible, look into that.


# new plan for using hyper

 - HTML processing thread:
    - get html from channel, grab urls, and throw those that are not within bloomfilter into reservoir.
    - if too slow: make several such threads.

 - CSS processing thread:
    - get css from channel, make nice, and if not within bloomfilter write to file.

 - Main IO loop thread:
    - grab lock for bloomfilter and urlreservoir, and get urls until reservoir is empty or I have gotten enougth.
    - get gotten urls asynchronous, send css through css channel and html through html channel.