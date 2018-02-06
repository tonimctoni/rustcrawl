# rustcrawl

Yet another webcrawler. This time with cool ideas as to how to avoid using very large hashtables.

I looked it up, it is called a bloom filter.

Also, the reservoir of urls to be crawled is finite. If full, inserts remove random previous elements, and crawler crawls less urls.

Also, it uses async IO, which is more elegant than a bunch of worker threads. It should scale better too, but this was not tested.

# conclusion

Rust does a superb job of managing memory consumption and processing time used. When it comes to making as many get requests as possible in as little time as possible, however, in my experience while making this project, it falls short. Hyper yielded better results than reqwest, but not as good as those I get from go. It is absolutely possible that this is due to mismanagement of resources from my part, as rust offers a great deal of freedom in this regard. Still, I will stick to go from now on for theese kinds of projects, at least for the forseable future.

I want to, again, point out the little cpu consumption I had while using rust for this project. Rust was absolutely superb in this regard.

# notes for myself

 - Storing strings instead of urls makes it more space efficient, but less time efficient (worth it).

<!--  - If all threads use the same seed (which I dont know if they would) that is not a problem, as long as statistical properties for each individual thread are ok. -->

 - Relaxed ordering for counters should be enought. Maybe ask someone who knows though. Then again, who does knows.

 - Using modulo to get random values within a range because rust's implementation of gen_range seems needlessly complicated to me.

<!-- # todo -->

<!--  - Add sleep at the beginning of each thread's loop. -->

<!--  - If reservoir is running out of space, the urls per crawl should be reduced.
    - Decided against it, actually. Revise later. -->

<!--  - Already used urls might get added to the reservoir, see if it is worth it to change that (use unique_gathered, unique_visited).
    - Actually, just not add urls to the reservoir that have been crawled already. Urls in reservoir might get discarded after all.
    - Actually actually, getting too many "url has been used" "errors", maybe use another unique.
    - Then again, a fast filling unique is not desired. -->

<!--  - Add a check/set in one method to unique. -->

<!--  - Add logger struct that logs errors.
    - Actually, maybe just print to stdout (or stderr). -->

<!--  - Use atomic counter to count gathered and visited pages. Maybe log this every so often. Also count css files. -->

<!--  - Use box syntax in unique module. -->

<!--  - Maybe use hyper, reqwest feels clunky. -->

<!--  - Remove #![allow(dead_code)]. -->

<!--  - Maybe use buffer_unordered. -->

<!--  - Maybe send uri to html_worker instead of string, if a move over channel is possible.
    - It is not: uri was moved at Client.get. -->

<!--  - Maybe use regex over Vec<u8> instead of string too (or not, its a move transform).
    - Not, it is a move transform. -->

<!--  - Maybe a direct transformation uri->url is possible, look into that.
    - Uri was moved, so there would be no point anyway. -->

<!--  - Probably need some sort of timeout. -->

<!--  - Replace unwraps with expects.
    - Actually, I see no point in that anymore. -->

<!--  - Limit the amount of crawled urls per site that share a host. Then remove MAX_URLS_PER_SITE.
    - Actually, just increase MAX_URLS_PER_SITE to like a thousand. -->

<!--  - Add timeout to getting chunks too. -->

<!--  - Make sure to use large bloom filter when deploying.

 - Document everything (as in, write comments and use rustdoc).
    - Also comment within functions.

 - Test further by using.

 - Make sure stuff works with only one thread too. So far so good.

 - Add timestampt to report. Maybe start timer when program starts and report on timer.

 - See if IO loop can be improved for performance.

 - Make sure magic numbers/strings are gone. -->

<!--  - Make sure url content that is not html or css gets discarded before it gets gotten by client. -->



<!-- 
# new plan for using hyper

 - HTML processing thread:
    - get html from channel, grab urls, and throw those that are not within bloomfilter into reservoir.
    - if too slow: make several such threads.

 - CSS processing thread:
    - get css from channel, make nice, and if not within bloomfilter write to file.

 - Main IO loop thread:
    - grab lock for bloomfilter and urlreservoir, and get urls until reservoir is empty or I have gotten enougth.
    - get gotten urls asynchronous, send css through css channel and html through html channel. -->