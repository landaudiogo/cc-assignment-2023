
To get your setup ready for the demo, there are a few changes you will have to
make to your infrastructure to make sure we correctly evaluate your setup:

1. The first change involves your setup to notify the `notifications-service`.

   When performing a request to the `notifications-service` you should add a
   new query parameter that is used to identify your group. For this purpose, a
   new file named `token` has been added to your group's one drive credentials
   folder. The contents of this file, are is the value of `token` query
   parameter you have to pass when notifying the `notifications-service`.

   Additionally, you also have to change the host you are sending the request,
   to `13.51.210.173:3000`.

   As such, when performing a request to the notifications-service, the url
   should look as follows: `http://13.51.210.173:3000/api/notify?token=<your-token>`.

   **You do not have to change the body of the request.**
1. Your REST API should be reachable at `<your-vm-ip>:3003`. This implies that
   when querying your REST API, the endpoints will be: 
   - Temperature:
     `http://176.34.72.62:3003/temperature?experiment-id=<experiment-id>&start-time=<start-time>&end-time=<end-time>`
   - Out-of-bounds:
     `http://176.34.72.62:3003/temperature/out-of-bounds?experiment-id=<experiment-id>``
1. Your prometheus instance should be reachable at `<your-vm-ip>:3008`. Make
   sure that your prometheus instance is exposing the metrics it is scraping
   from the `node_exporter`. 

   Note: you only need the prometheus service from the local setup
   `docker-compose` file running, i.e., you don't need to have the grafana
   grafana service running.
