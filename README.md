# shorty

[![Latest version](https://img.shields.io/crates/v/shorty.svg)](https://crates.io/crates/shorty)
[![Build Status](https://travis-ci.org/ffissore/shorty.svg?branch=master)](https://travis-ci.org/ffissore/shorty)
![License](https://img.shields.io/github/license/ffissore/shorty.svg)

Shorty is a URL shortener: it assigns a short ID to a URL of any length, and when people will access the URL with that short ID, they will be redirected to the original URL.

This is useful in cases such as sending SMS notifications, when you have a limited number of characters and don't want to waste them with a long URL and its parameters.

Shorty stores its data on Redis.

You can see it in action at https://with.lv/

### Multiple ways of deploying it

Shorty is available as

- a [rust library](#rust-library)
- an [http microservice](#http-microservice)
- an [AWS lambda](#aws-lambda)
- an [Azure function](#azure-function)

### Rust library

Shorty is written in rust, and available as a crate library.

For additional information, take a look at the [documentation](https://docs.rs/shorty), and use shorty-http binary crate as an example. 

### HTTP microservice

Shorty stores its data on redis, so you need to install redis first. How to do that depends on your operating system. If you are on a debian like linux distro, it's just a
```bash
apt-get install redis
```
If you prefer `docker`, it's
```bash
docker run --rm -it -p 6379:6379 redis:alpine
```

Once redis is ready, download the latest shorty [release](https://github.com/ffissore/shorty/releases), unpack the archive and run shorty with

```bash
SHORTENER_API_KEY_MANDATORY=false ./shorty-http
```

Shorty will log `Starting server on 127.0.0.1:8088`.

Now scroll down to [Using shorty](#using-shorty).

### AWS lambda

In order to deploy Shorty on AWS, you need... node.js. Duh! Yeah, it's a shame, but `serverless` is a node package and de-facto standard for deploying lambdas on AWS, and it works well.

Install the required dependencies with 

```bash
npm install
```

Then run

```bash
serverless deploy
```

This will download a docker image that will be used to compile shorty. It will then create a Redis instance on AWS, all the networking bits required to make the lambda connect to Redis, and finally it will deploy the lambda.

When done, it will print something like

```
endpoints:
  GET - https://9tgwceucu4.execute-api.us-east-1.amazonaws.com/dev/{key}
  POST - https://9tgwceucu4.execute-api.us-east-1.amazonaws.com/dev/
```

Please bear in mind that:
1. Your AWS account will be charged: lambdas are free up to 1 million requests but the "NAT gateway" required to expose them is not
1. The bloat of CloudFormation you'll find in `serverless.yml` has been copy-pasted (thx [ittus](https://github.com/ittus/aws-lambda-vpc-nat-examples/blob/master/serverless.yml)) and I barely understand what it does.

When you're done playing with shorty, delete everything with
```bash
serverless remove
```   

Now scroll down to [Using shorty](#using-shorty).

### Azure function

Azure does not support Rust directly, but they allow you to run any kind of runtime as long as it runs in docker.

Shorty docker image is available at [docker hub](https://hub.docker.com/r/ffissore/shorty).

The following commands were taken from [this Azure guide](https://docs.microsoft.com/azure/app-service/containers/tutorial-custom-docker-image).

* Create a resource group
```bash
az group create --name shorty-resources --location "West Europe"
```
* Create a redis instance  
```bash
az redis create --resource-group shorty-resources --name shorty-redis --location "West Europe" --sku Basic --vm-size c0 --enable-non-ssl-port
```
* Create a service plan
```bash
az appservice plan create --name shorty-service-plan --resource-group shorty-resources --sku B1 --is-linux
``` 
* Create a webapp
```bash
az webapp create --resource-group shorty-resources --plan shorty-service-plan --name shortyshorty --deployment-container-image-name ffissore/shorty:latest
```
* Tell azure that Shorty listens on port 8088
```bash
az webapp config appsettings set --resource-group shorty-resources --name shortyshorty --settings WEBSITES_PORT=8088
```
* and that for now we don't need API keys
```bash
az webapp config appsettings set --resource-group shorty-resources --name shortyshorty --settings SHORTENER_API_KEY_MANDATORY=false
```
* Now locate your redis instance on azure portal, and copy its primary access key. It will something like `RandomString=`: that's redis password. Set the redis host variable accordingly
```bash
az webapp config appsettings set --resource-group shorty-resources --name shortyshorty --settings SHORTENER_REDIS_HOST=:RandomString=@shorty-redis.redis.cache.windows.net
```

Shorty will be available at https://shortyshorty.azurewebsites.net/

Please bear in mind that your account will be charged for all of the above.

When you're done playing with shorty, delete everything with
```bash
az group delete --name shorty-resources
```   

Now scroll down to [Using shorty](#using-shorty).
 
### Using shorty

The following instructions assume shorty is running on your pc. If that's not the case, replace `http://localhost:8088` with the proper domain.
 
Try this `curl` to store a URL

```bash
curl -vv http://localhost:8088/ -H 'Content-Type: application/json' --data '{"url":"https://en.wikipedia.org/wiki/URL_shortening#Techniques"}'
```

It will output something like

```json
{"id":"CGQ6LM8bfj","url":"https://en.wikipedia.org/wiki/URL_shortening#Techniques"}
```

Now try resolving that ID

```bash
curl -vv http://localhost:8088/CGQ6LM8bfj
```

The output headers of curl will contain a `Location: https://en.wikipedia.org/wiki/URL_shortening#Techniques`. Try opening the shorty url with your browser.

### Configuration

Shorty can be configured through environment variables

* `SHORTENER_REDIS_HOST`: the host of the redis server, defaults to 127.0.0.1
* `SHORTENER_REDIS_PORT`: the port of the redis server, defaults to 6379
* `SHORTENER_API_KEY_MANDATORY`: do users have to provide an API key in order to create a new short URL? boolean, defaults to true
* `SHORTENER_RATE_LIMIT`: the amount of new short url a single API key can create in a period, defaults to 10, if set to 0 no limit is applied
* `SHORTENER_RATE_LIMIT_PERIOD`: the period of the rate limit, if active, defaults to 600 seconds (10 mins)
* `SHORTENER_ID_LENGTH`: the length of the ID generated for each URL, defaults to 10. The char set is `a-zA-Z0-9` = 62 chars. If you plan to use shorty only internally, you can use a much shorter ID, like 4 chars.
* `SHORTENER_HOST`: the host shorty will listen to
* `SHORTENER_PORT`: the port shorty will listen to

### What's on Redis

* API keys: they are prefixed with `API_KEY_`, stored as `API_KEY_my_api_key`, and assigned a boolean value. A missing API key or an API key assigned to `false` will return error "Invalid API key"
* Call rate keys: they are prefixed with `RATE_`, stored as `RATE_my_api_key`, and assigned the registered number of calls. The key is valid until `rate limit period` (see paragraph above) is over.
* Short IDs, at the configured length (see example above): they are assigned to the original URL
