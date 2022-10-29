# fabriq

A developer experience and GitHub first engineering system.

## Install CLI

One line instructions to install the CLI...

```
$ curl http://.... | bash
```

## Login

Create a Github personal access token for your account and use the `fabriq` cli to login. Future `fabriq` commands will execute in the context of this PAT and its team memberships etc.

```
$ fabriq login PAT
```

## Target Setup

Let's add a couple of targets to our system. Targets allow us to place deployments to hosts whose labels match.

```
$ fabriq target create eastus2 --label region:eastus2
$ fabriq target create cncf-infra-stable --label infra:stable
```

## Template Setup

Let's add some templates to our system.

```
$ fabriq template create external-service --ref main --path external-service --repo git@github.com:fabriq-cloud/templates
$ fabriq template create cncf-infra-stable --ref main --path cncf-infrastructure --repo git@github.com:fabriq-cloud/templates
$ fabriq template create cncf-observability-stable --ref main --path cncf-observability --repo git@github.com:fabriq-cloud/templates
$ fabriq template create cncf-observability-beta --ref beta --path cncf-observability --repo git@github.com:fabriq-cloud/templates
```

## Platform Deployments

We next want to tell `fabriq` to deploy CNCF infrastructure (Contour, Linkerd, Fluentbit) to all of the hosts added to the system that match the `infra-stable` target.

```
$ fabriq workload create cncf-infra --team my-org/platform --template cncf-infra-stable
$ fabriq deployment create stable --workload cncf-infra --hosts all --target cncf-infra-stable --team fabriq-cloud/platform
```

## Host Setup

Let's add our first host to our system. Hosts receive and execute deployments.

```
$ fabriq host create azure-eastus2-1 --label region:eastus2 --label infra:stable
```

Because it matches the `infra-stable` target above, this host will automatically have the `stable` deployment of `infra` assigned to it.

## Cross Team Observability

As a team, we use CNCF observability, and we want to deploy this such that it is accessible by any of the services we deploy, and so that any team member can access it to understand and ask questions about how our services are performing.

```
$ fabriq workload create observability --team hello
$ fabriq deployment create stable --workload observability --target eastus2 --hosts 1 --template cncf-observability-stable
```

## Template Service from Seed

Let's template out a simple service from a Github template. A platform team might maintain this seed in collaboration with service teams to help create new services that utilize engineering fundamentals appropriately.

```
$ fabriq workload init hello-service --seed fabriq-cloud/rust-service-api hello-service
```

Doing this on the CLI makes it easy to describe, but we could have just as easily have templated it from GitHub itself.

## Deploy by Default

As part of the templating of the workload, the tool printed out a url:

```
https://main.hello-service.my-team.fabriq.cloud
```

If we wait a few minutes, we should be able to go to this url and receive

```
Hello World!
```

How did this work?

The `rust-service-api` template we created our workload from includes a GitHub Action to build, containerize, and deploy
the application.

When `hello-service` is created in our team's organization, this Github Action is run which includes the two operations:

```
$ fabriq workload create hello-service --team hello --template external-service
```

This registers this service (since it hasn't been created before), specifying that we would like deployments of this service to, by default, use the `external-service` deployment template.

By default, it uses your user group for this service, but alternatively you can use `--group {group}` to specify the group to use for the service. It also creates an .fabriq/workload.yaml and adds details about this service (name, group, deployment template). (too much detail for now?)

```
$ fabriq deployment create main --workload hello-service --affinity observability/stable --target eastus
```

This creates the `hello-service` workload if it hasn't already been created and a `main` deployment for it on a host that matches the `eastus` target. Behind the scenes, `fabriq` orchestrates assigning this workload to a host and rolling that workload out to the host.

TODO: Can we use a service operator within the `external-service` template to automatically point DNS to the host it is configured on?
TODO: Use a host probe to identify the ingress IP address such that Fabriq knows it? Or can we just establish a CNAME for the host and then point the deployment to that CNAME?

## Deploy node.js application

We could have named this deployment by adding a `name` parameter, but by default `fabriq` will choose the name of the current branch of our Git repo.

`[--workload hello-service]` is assumed in the above because of you are running the command in the `hello-service` service repo and Fabriq will pull defaults from `.fabriq/service.yaml`.

Likewise, since we didn't override them, the deployment will inherit the same deployment template and target from the service. This is usually what you want, but you can override them, if, for example you have a very large production deployment or very small dev deployment that you want to do. (too much detail?)

Behind the scenes this will create a deployment for the service, matching it to a host that matches our `eastus` target, and because
we used an `external-service` deployment template, and will surface it on `main.hello-world.timfpark.fabriq.cloud` as a specific example
of the general form `{deployment}.{service}.{team}.{org}.fabriq.cloud`.

Additionally, each time that we push a commit to our `main` branch, our GitHub CI will build our service's container, and assuming its test pass, update our `main` deployment so that we can test it.

## Container Promotion

For production deployments you don't want the build of a container to immediately be deployed. Instead service teams typically test a build in another environment and then promote it in a copy exactly manner (no new container build) to production.

Let's first create a `prod` deployment for our workload that we can promote our `main` builds to production:

```

$ fabriq deployment create prod
deployment created:
name: prod
service: hello-service
template: external-service (inherited from service)
target: eastus (inherited from service)
group: timfpark

```

In this case we are specifying the name `prod` explicitly, but `fabriq` will default to all of the previous settings.

This won't trigger a deployment because there is no `image` config specified and the `external-service` template requires it.

And then we can promote our `main` development build to production with:

```

$ fabriq deployment promote main prod

```

This copies the image tag from `main` deployment and applies it as config to the `prod` deployment, triggering the first deployment of `prod` and surfacing it on `prod.hello-world.timfpark.fabriq.network`.

## Dialtone

```

$ fabriq workload create contour --template contour (ingress for group)
$ fabriq deployment create contour-prod --target prod --template-branch main

```

## Observability

Deployment is only the first step in managing a production service. We also want to be able to watch metrics from our service.

The platform deploys a common set of observability tools for our workloads.

Metrics are backhauled per group to central storage
Just start with this being the single cluster the group's apps are deployed on

```

$ fabriq deployment proxy grafana

```

This opens a browser window and directs you to Grafana where you can access your metrics. In this case, we have a
set of metrics dashboards for our service...

TODO: How do we provision metrics dashboards for our application?
TODO: Can we query on just the specific deployment in grafana automatically?
TODO: Can we configure the app to label metrics with the branch our deployment is in?

## Logs

TODO: What to use for logs?

```

$ fabriq logs

```

## Tracing

TODO: Now do we route to the jaeger instance for the application?

Want to access the Jaeger statistics

```

$ fabriq deployment proxy hello-service tracing

```

## Seed walkthrough

- Run It
- Show it prints "Hello {name}" in response to incoming query.
- Show it includes Github Action to build container
- Show it includes metrics and logging (v2)

## Logical Model of Walkthrough

```
  -- hosts
    -- azure-eastus2-1
      -- deployments
        -- infra/stable
        -- observability/stable
        -- hello-world/main
        -- hello-world/feature
  -- teams
    -- platform
      -- workloads
        -- infra
          -- deployments
            -- stable
              -- contour
              -- fluentbit
              -- linkerd
     -- hello
      -- workloads
        -- observability
          -- deployments
            -- stable
              -- grafana
              -- prometheus
              -- jaeger
            -- beta

        -- hello-world
          -- deployments
            -- main
            -- feature
```
