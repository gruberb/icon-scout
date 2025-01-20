# IconScout

- IconScout is a web service which is opening a POST endpoint on `/favicons`, which accepts a `JSON` file with a list of websites.
- It tries to fetch favicons from the given list of websites
- If found, it stores them in a Google Cloud bucket
- It then returns a response, sutiable for the [`icon-scout-frontend`](https://github.com/gruberb/icon-scout-frontend) project.
