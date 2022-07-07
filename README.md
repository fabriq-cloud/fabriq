# akira

A PaaS for Multicloud 

Let's do a walkthrough to best understand how it works

## Install CLI

One line instructions to install the CLI...

## Login

```
$ akira login timfpark@gmail.com
```

Logs in with Github
Creates PAT for CLI and stores it locally?
Automatically creates a group for the user and makes it the default?

## Seed sample node.js application

Let's use a sample node.js application to see how easy it is to deploy. 

Let's template out a simple service from a Github template:

```
$ akira service template hello-service --from microsoft/nodejs-service-api
```

This is just a convience function and we could have templated it from GitHub itself. 

## Seed walkthrough

- Run It
- Show it prints "Hello {name}" in response to incoming query.
- Show it includes Github Action to build container
- Show it includes metrics and logging (v2)

## Deploy node.js application

Let's deploy it. First, we want to register our service:

```
$ akira service create hello-service --template external-service --target eastus
```

This registers this service, specifying that we would like deployments of this service to, by default, use the `external-service` deployment template and place these deployments on hosts matching `eastus`.

By default, it uses your user group for this service, but alternatively you can use `--group {group}` to specify the group to use for the service. It also creates an .akira/workload.yaml and adds details about this service (name, group, deployment template). (too much detail for now?)

`akira` enables you to make multiple deployments of your service so you can progressively roll out changes.  Let's make our first one now:

```
$ akira deployment create
deployment created:
   name: main (default from git branch)
   service: hello-service
   template: external-service (inherited from service)
   target: eastus (inherited from service)
   group: timfpark
```

We could have named this deployment by adding a `name` parameter, but by default `akira` will choose the name of the current branch of our Git repo.

`[--service hello-service]` is assumed in the above because of you are running the command in the `hello-service` service repo and Akira will pull defaults from `.akira/service.yaml`.

Likewise, since we didn't override them, the deployment will inherit the same deployment template and target from the service. This is usually what you want, but you can override them, if, for example you have a very large production deployment or very small dev deployment that you want to do. (too much detail?)

Behind the scenes this will create a deployment for the service, matching it to a host that matches our `eastus` target, and because 
we used an `external-service` deployment template, and will surface it on `main.hello-world.timfpark.akira.network` as a specific example 
of the general form `{deployment}.{service}.{group}.akira.network`.

TODO: Can we use a service operator within the `external-service` template to automatically point DNS to the host it is configured on?
TODO: Use a host probe to identify the ingress IP address such that Akira knows it?  Or can we just establish a CNAME for the host and then point the deployment to that CNAME?

Additionally, each time that we push a commit to our `main` branch, our GitHub CI will build our service's container, and assuming its test pass, update our `main` deployment so that we can test it.

## Container Promotion

For production deployments you don't want the build of a container to immediately be deployed. Instead service teams typically test a build in another environment and then promote it in a copy exactly manner (no new container build) to production.

Let's first create a `prod` deployment for our workload that we can promote our `main` builds to production:

```
$ akira deployment create prod
deployment created:
   name: prod
   service: hello-service
   template: external-service (inherited from service)
   target: eastus (inherited from service)
   group: timfpark
```

In this case we are specifying the name `prod` explicitly, but `akira` will default to all of the previous settings.

This won't trigger a deployment because there is no `image` config specified and the `external-service` template requires it.

And then we can promote our `main` development build to production with:

```
$ akira deployment promote main prod
```

This copies the image tag from `main` deployment and applies it as config to the `prod` deployment, triggering the first deployment of `prod` and surfacing it on `prod.hello-world.timfpark.akira.network`.

## Metrics

Deployment is only the first step in managing a production service.  We also want to be able to watch metrics from our service.

Akira automatically provisions observability tools for our service, including Prometheus and Grafana for metrics.  We can access them via
the command line by executing in our repo:

```
$ akira metrics
```

This opens a browser window and directs you to Grafana where you can access your metrics.  In this case, we have a 
set of metrics dashboards for our service...

TODO: How do we provision metrics dashboards for our application?
TODO: Can we query on just the specific deployment in grafana automatically?
TODO: Can we configure the app to label metrics with the branch our deployment is in?

## Logs

TODO: What to use for logs?

```
$ akira logs
```

## Tracing

TODO: Now do we route to the jaeger instance for the application?

```
$ akira tracing
```
