# Learning Objectives

- Cloud Systems
- Virtual Machines
- Distributed Systems
- Asynchronous communication
- Synchronous Communication
- Scalable Architectures
- Monitoring
- Real-time processing
- Batch processing

# Problem Context

XYZ Laboratories is a research lab that employs around 10,000 scientists, ranging from chemistry, biology and physics.

Their facilities contain multiple labs wherein each scientist can run experiments for their research. Some of the experiments carried out require very specific monitoring over the temperature values throughout the whole experiment.

Related to these experiments, this company has hired your team to develop a `Temperature Observability` microservice which has 2 main responsibilities:
- To provide a historic trace of the temperature values throughout an experiment.
- To notify the researcher when the temperature values are not within a pre-defined range for that experiment. 

## Experiment Process

The research centre has many physical spaces available for the temperature-controlled experiments. These spaces can be of any size, and therefore the researcher has to set the environment so as to have the right amount of sensors for the physical space required for the experiment. This also means that for each experiment, there might be an arbitrary number of sensors monitoring the temperature. The physical space's temperature in these conditions is defined as the average temperature of all sensors allocated to an experiment.

Depending on the granularity of an experiment's temperature control, each experiment has its own sampling rate, e.g. one experiment might have a sampling rate of 1 measurement per second, and another has a sampling rate of 1 every 0.1 seconds.

It is fundamental that the physical space where the experiment is to be carried out has reached its desired temperature. The desired range is specific to each experiment and defined by the researcher while configuring the experiment. 

Lastly, the experiment is carried out for as long as the researcher desires. However whenever the temperature for an experiment falls out of range, the researcher has to be notified.

The stages are:

1. Experiment configuration phase: This is where the researcher configures the requirements for the experiment, and involves specifying: Sensors that will track the temperature of the physical space; The allowed temperature range for the experiment; and the researcher's email address for notifications;
2. Stabilization phase: The temperature is being controlled to reach the values within the range specified by the researcher (the experiment hasn't yet started at this stage);
3. Carrying out the experiment: Temperature measurements are expected to be within the range specified in the configuration phase (otherwise the researcher has to be notified).
4. Experiment termination: The researcher informs that the experiment is no longer running, and no more temperature measurements are produced.

Your microservice will have to keep track of several experiments running concurrently, each with their own temperature ranges, sensors, duration and sampling rate.

## Functional Requirements

- [ ] The system should allow a researcher to configure an experiment.
- [ ] The system should provide a REST endpoint that allows querying the time-intervals during which the temperature fell out of the defined temperature range for an experiment.
- [ ] The system should notify the “Notifications” microservice via a REST endpoint when the temperature value for an experiment is out of the defined temperature range.
- [ ] The implemented microservice should have a monitoring component that provides an overview of the microservice’s costs over time.
- [ ] The system should be able to scale based on the system's load.

## Non-Functional Requirements

- [ ] The system should notify the “Notifications” service within 10 seconds from when the temperature values deviate for each experiment given there are no more than 100 experiments running concurrently each with a sampling rate of $1s$.

# Technical Details

Your microservice has to monitor all temperature experiments carried out throughout the XYZ research centre.

The following figure provides an overview of the system:

![System Overview](https://github.com/landaudiogo/cc-assignment-2023/assets/26680755/8792e516-fc37-41a8-bb14-61bf20121be0)

The service your team will have to implement, is the one highlighted in yellow. There are 2 main interfaces your microservice has to provide which are highlighted as annotations 1 and 2:
- Interface 1 - Kafka consumer that reads data from a Kafka topic where the data regarding an experiment's procedure is produced. The information is produced to the `<team_id>` topic, where `<team_id>` is an identifier unique to each team.
- Interface 2 - Temperature Observability REST API your service will have to provide for researchers to consult the internal state of their experiments. The main queries will be: the temperature values for a specified time-interval; The time instants throughout an experiment wherein the temperature fell outside of the range specified in the experiment's configuration.
- Interface 3 - Notifications REST API provided by the notification service which is **already implemented**. Your service will only have to interact with it to communicate any event to a researcher.

![Line Chart Events drawio](https://github.com/landaudiogo/cc-assignment-2023/assets/26680755/904794dc-b6af-44af-ac9c-f33d78cc1e8f)

A service's interface is what exposes its functionalities to users or services that can communicate with it. Your team's assignment consists of designing and implementing a system that complies with the functional and the non-functional requirements while exposing the aforementioned interfaces. Additionally, as long as your services are running as containers, your team has the liberty to choose any service, or any programming language to develop the Temperature Observability Microservice.

## Interfaces

To illustrate the different events throughout an experiment, we will use an example wherein an experiment requires the temperature to be between 25-26 °C. The following diagram contains a representation of the event's lifecycle.



Annotation `1` marks the moment an `Experiment Configuration` event was published. This event is communicated by the researcher to indicate which temperature sensors are to be used during the experiment, and an experiment's temperature range.

Annotation `2` marks the moment `Stabilization Started` event was published. This event is communicated by the researcher to indicate when the temperature controller is controlling the physical space's temperature to be within the desired range. Starting from annotation `2-6`, temperature measurements are sent to the `<team_id>` topic.

Annotation `3` marks the moment the temperature in the experiment's physical space has reached the desired range specified in the experiment configuration event.

Annotation `4` marks the moment the `Experiment Started` event was published. This event indicates the moment the researcher has started the experiment. From this moment onward, your microservice has to be aware of when the experiment's temperature falls out-of-range to notify the main researcher.

Annotation `5` marks the moment the experiment's temperature falls out-of-range, which requires notifying the researcher through the notifications service. **The remaining measurements that are out-of-range until the temperature stabilizes again do not have to be notified, only the measurement taken at Annotation `5`.**

Annotation `6` marks the moment the `Experiment Terminated` event is published indicating the end of the experiment. From this moment onward, no more measurement or data referring to that experiment is sent to the  `<team_id>` topic, but queries related to the experiment's temperature values can be performed via the Temperature Observability REST API.

### Topic Events

The XYZ research centre manages a kafka cluster that contains a topic with the name `<team_id>` that contains data related to the state of an experiment. In Kafka’s context, a topic is where events are published to and consumed from by data producers and consumers respectively. As such, this is also the data your microservice will have to consume to track each experiments' state.

Each event published into the `<team_id>` topic is serialized using the avro format schema. The following sections will describe each event and provide an example payload and schema for each event.
#### Experiment Configuration Event

This event configures an experiment and involves specifying: Sensors that will track the temperature of the physical space; The allowed temperature range for the experiment; and the researcher's email address for notifications

As such, if the main researcher for experiment `9ee55bd4-a531-409c-9a64-0398353cadc5` has the email `d.landau@uu.nl`, and the physical environment has been setup to contain the sensors with ids `["66cc5dc0-d75a-40ee-88d5-0308017191af", "ac5e0ea2-a04d-4eb3-a6e3-206d47ffe9e1"]`, and the maximum allowed temperature is `26` and the minimum temperature is `25`, then the configuration event would look like:

```json
{
	"experiment": "9ee55bd4-a531-409c-9a64-0398353cadc5",
	"researcher": "d.landau@uu.nl",	
	"sensors": [
		"66cc5dc0-d75a-40ee-88d5-0308017191af", 
		"ac5e0ea2-a04d-4eb3-a6e3-206d47ffe9e1"
	],
	"temperature_range": {
		"upper_threshold": 26.0,
		"lower_threshold": 25.0,
	}
}
```

The researcher field contains the email of the researcher that has to be notified by the notifications service.

Schema: 
```json
{
    "type": "record", 
    "name": "ExperimentConfig", 
    "fields": [
        {
            "type": "string",
            "name": "experiment"
        },
        {
            "type": "string",
            "name": "researcher"
        },
        {
            "name": "sensors", 
            "type": {
                "type": "array",
                "items": "string"
            }
        }, 
        {
            "name": "temperature_range",
            "type": {
                "type": "record",
                "name": "temperature_range",
                "fields": [
                    {"name": "upper_threshold", "type": "float"},
                    {"name": "lower_threshold", "type": "float"}
                ]
            } 
        }
    ]
}
```

#### Stabilization Started Event

This event signals the time at which the experiment's physical space's temperature started being controlled. After this event has been published to the topic, `Sensor Temperature Measured` Event events will follow. For this reason, it is also the moment your microservice has to start monitoring the temperature values for an experiment. 

**The temperature values throughout the stabilization phase do not have to be stored for historic querying**. However, because the researcher has to be notified when the temperature has reached the range indicated in the configuration event, the temperature has to be monitored by your microservice.

An example payload for this event:
```json
{
	"experiment": "9ee55bd4-a531-409c-9a64-0398353cadc5",
	"timestamp": 1691419380.9467194
}
```

Schema:
```json
{
    "type": "record", 
    "name": "stabilization_started", 
    "fields": [
        {
            "type": "string",
            "name": "experiment"
        },
        {
            "name": "timestamp", 
            "type": "double"
        }
    ]
}
```
#### Experiment Started Event

This event marks the time at which the researcher has started the experiment. `Sensor Temperature Measured` events consumed after this moment have to be stored for further consultation by a researcher via the Temperature Observability REST API. 

Example event: 
```json
{
	"experiment": "9ee55bd4-a531-409c-9a64-0398353cadc5",
	"timestamp": 1691419385.9467194
}
```

Schema: 
```json
{
    "type": "record", 
    "name": "experiment_started", 
    "fields": [
        {
            "type": "string",
            "name": "experiment"
        },
        {
            "name": "timestamp", 
            "type": "double"
        }
    ]
}
```
#### Sensor Temperature Measured Event

An experiment's physical space temperature is the average temperature measured by all the sensors allocated to an experiment. Resorting to the example provided in the `Experiment Configuration`, let us consider that we have 2 sensors allocated to our experiment, `66cc5dc0-d75a-40ee-88d5-0308017191af` and `ac5e0ea2-a04d-4eb3-a6e3-206d47ffe9e1`. To calculate the experiment's measured temperature at timestamp `1691419376.9467194`, we need 1 measurement per sensor. If for sensor `66cc5dc0-d75a-40ee-88d5-0308017191af` we get the event:
```json
{
	"experiment": "9ee55bd4-a531-409c-9a64-0398353cadc5",
	"sensor": "66cc5dc0-d75a-40ee-88d5-0308017191af",
	"measurement-id": "fb5b8af2-9309-46c8-bcbe-87c98b407c3d",
	"timestamp":  1691419390.9467194, 
	"temperature": 25.5, 
	"measurement_hash": "R8n76xYE4v/AUk1X5hM/+kkLHH5KYdoDpKiz7dUxybXaq++DcjXcuqM4GxNFg/jbvjmTnS/rh7FKoXvjJu1sg4Gc/cELVkDJ+ZWl0HTS81AfyQQmFH/CID53T3ynTtFmYATtWCnGxWiHffo/RFVSNXdQQvb2x5YBFA4DX7mznPpaC3qzwtzGEGgYtkDkzS0cVC4Kd5gWgJwInx7SHBIoflHZvfzUi329vIU"
}
```
and for sensor `ac5e0ea2-a04d-4eb3-a6e3-206d47ffe9e1`, the event: 
```json
{
	"experiment": "9ee55bd4-a531-409c-9a64-0398353cadc5",
	"sensor": "ac5e0ea2-a04d-4eb3-a6e3-206d47ffe9e1",
	"measurement-id": "fb5b8af2-9309-46c8-bcbe-87c98b407c3d",
	"timestamp":  1691419390.9467194, 
	"temperature": 25.3, 
	"measurement_hash": "R8n76xYE4v/AUk1X5hM/+kkLHH5KYdoDpKiz7dUxybXaq++DcjXcuqM4GxNFg/jbvjmTnS/rh7FKoXvjJu1sg4Gc/cELVkDJ+ZWl0HTS81AfyQQmFH/CID53T3ynTtFmYATtWCnGxWiHffo/RFVSNXdQQvb2x5YBFA4DX7mznPpaC3qzwtzGEGgYtkDkzS0cVC4Kd5gWgJwInx7SHBIoflHZvfzUi329vIU"
}
```
the temperature measured in the experiment's physical space is $(25.3 + 25.5)/2 = 25.4$. This final value ($25.4$), is the value that has to be compared with the range provided in the experiment configuration to determine whether the researcher has to be notified. This is also the value that has to be stored as the experiment's temperature at timestamp `1691404551.541` when a researcher queries for the experiment's temperature values via the Temperature Observability REST API. 

As shown in the previous 2 measurements, both have the same `measurement-id` field. This is important when communicating to the notification service the ID of the measurement being notified. This field is used by the Notification Service to calculate your service's notification latency.

Schema: 
```json
{
    "type": "record", 
    "name": "sensor_temperature_measured", 
    "fields": [
        {
            "name": "experiment",
            "type": "string"
        },
        {
            "name": "sensor",
            "type": "string"
        },
        {
            "name": "measurement_id",
            "type": "string"
        },
        {
            "name": "timestamp", 
            "type": "double"
        },
        {
            "name": "temperature", 
            "type": "float"
        }, 
        {
            "name": "measurement_hash", 
            "type": "string"
        }
    ]
}
```
#### Experiment Terminated Event

This event marks the end of an experiment. No more temperature measurements are received after publishing this event. 

Example event: 
```json
{
	"experiment": "9ee55bd4-a531-409c-9a64-0398353cadc5",
	"timestamp": 1691419395.9467194
}
```

Schema: 
```json
{
    "type": "record", 
    "name": "experiment_terminated", 
    "fields": [
        {
            "type": "string",
            "name": "experiment"
        },
        {
            "name": "timestamp", 
            "type": "double"
        }
    ]
}
```

### Temperature Observability REST API
#### Experiment Out-of-Range Endpoint
##### Description
Given an experiment as input, this endpoint should return the list of measurements during which an experiment was out of the temperature range specified in the experiment configuration event.
##### Request 
GET `/temperature/out-of-range`

**Query parameters**:
- `experiment-id` (`string`): The ID of the experiment.

**Example Request**:
```bash
curl -X GET http://<your-service>/temperature/out-of-range -G \
    -d "experiment-id=9ee55bd4-a531-409c-9a64-0398353cadc5"
```
##### Response
HTTP status code: 200

**Example Response**: 
```json
[
  {"timestamp": 1691419390.9467194, "temperature": 25.4},
  {"timestamp": 1691419391.9467194, "temperature": 25.4},
  {"timestamp": 1691419392.9467194, "temperature": 25.3},
  {"timestamp": 1691419393.9467194, "temperature": 25.5},
  {"timestamp": 1691419394.9467194, "temperature": 25.5}
]
```

JSON Schema: 
```json
{
  "title": "out-of-range",
  "type": "array",
  "items": {
    "title": "point",
    "type": "object", 
    "properties": {
    	"timestamp": {"type": "number"},
        "temperature": {"type": "number"}
    }
  }
}
```
#### Experiment Temperature Endpoint
##### Description
Given an experiment, a start-time and an end-time, this endpoint should return the temperature measurements for the specified time-interval.
##### Request

GET `/temperature`

**Query parameters**:
- `experiment-id` (`string`): The ID of the experiment.
- `start-time` (`double`): epoch timestamp for the interval's start time.
- `end-time` (`double`): : epoch timestamp for the interval's end time.

**Example Request**:
```bash
curl -X GET http://<your-service>/temperature/out-of-range -G \
    -d "experiment-id=9ee55bd4-a531-409c-9a64-0398353cadc5" \
    -d "start-time=1691419390.000000" \ 
    -d "end-time=1691419395.000000"
```
##### Response 
HTTP status code: 200

**Example Response**. 
```json
[
  {"timestamp": 1691419390.9467194, "temperature": 25.4},
  {"timestamp": 1691419391.9467194, "temperature": 25.4},
  {"timestamp": 1691419392.9467194, "temperature": 25.3},
  {"timestamp": 1691419393.9467194, "temperature": 25.5},
  {"timestamp": 1691419394.9467194, "temperature": 25.5}
]
```

JSON Schema: 
```json
{
  "title": "temperature",
  "type": "array",
  "items": {
    "title": "point",
    "type": "object", 
    "properties": {
    	"timestamp": {"type": "number"},
        "temperature": {"type": "number"}
    }
  }
}
```

### Notifications API

This is the service you will have to notify when communicating certain events related to an experiment. You will not have to implement this interface, but you will have to interact with it, and as such, this section describes the notification service REST API interface.
#### Notify Endpoint
##### Description
Notifies a researcher of a temperature stabilized or an out-of-range temperature measurement event.
##### Request

POST `/notify`

Body Parameters: 
- `notification_type`: An enum which indicates the type of notification to be
  sent to the researcher (Stabilized/OutOfRange).
- `researcher`: The researcher's email to be notified. 
- `experiment_id`: The experiment's ID to which the measurement belongs.
- `measurement_id`: The temperature measurement that concerns this
  notification.
- `cipher_data`: This value represents the `measurement_hash` value in a
  measurement’s event payload. Through the data contained in this value, the
  notifications service determines whether it was correctly notified of a
  stabilized or out-of-range event.

**Body Schema**: 
```json
{
    "title": "notify-body",
    "type": "object", 
    "properties": {
    	"notification_type": {"type": "string", "enum": ["OutOfRange", "Stabilized"]},
        "researcher": {"type": "string"},
        "experiment_id": {"type": "string"},
        "measurement_id": {"type": "string"},
        "cipher_data": {"type": "string"}
    }
}
```

**Example Request**:
```bash
curl -X 'POST' \
  'http://localhost:3000/api/notify' \
  -H 'accept: */*' \
  -H 'Content-Type: application/json; charset=utf-8' \
  -d '{
       "notification_type": "OutOfRange", 
       "researcher": "d.landau@uu.nl",
       "measurement_id": "1234", 
       "experiment_id": "5678", 
       "cipher_data": "D5qnEHeIrTYmLwYX.hSZNb3xxQ9MtGhRP7E52yv2seWo4tUxYe28ATJVHUi0J++SFyfq5LQc0sTmiS4ILiM0/YsPHgp5fQKuRuuHLSyLA1WR9YIRS6nYrokZ68u4OLC4j26JW/QpiGmAydGKPIvV2ImD8t1NOUrejbnp/cmbMDUKO1hbXGPfD7oTvvk6JQVBAxSPVB96jDv7C4sGTmuEDZPoIpojcTBFP2xA"
}'
```
##### Response 
HTTP status code: 200

Payload: 
```json
1470.385802745819
```

Payload Schema:
```json
{
    "title": "notify-response-body",
    "type": "number", 
}
```

# Development Setup

Your team will have access to a single VM on the cloud where you will be deploying your services. 

We will provide you with the other external services so you can test how your system is behaving in a controlled environment. These will be: 
- A Kafka topic which only your group has access to. Your group will have full control over the topic.
- A load generator for your kafka topic that publishes the streams of events for an experiment.
- A notification service for your microservice to notify a researcher.

# Evaluation

The assignment evaluation will consist of 3 components:
- Demo 
- Architecture Assessment
- Report
## Demo

Your implementation will be put to test during a Demo Session to be scheduled at the end of the period.

The demo will be split into 3 different parts:
- Verification of data consistency (endpoint integration testing): Load will be generated for your microservice to consume, followed by querying the rest endpoints provided by your service to evaluate the consistency of the data presented by your team's microservice.
- Stress test the architecture endpoint + event entrypoints' scalability (implementation scalability): Evaluates how your architecture performs under varying load intensities generated by our workloads. The workload patterns will not be shared beforehand, and therefore, the architecture is expected to be flexible to different workload intensities. We will evaluate your microservice's performance based on:
	1. Endpoint response time
	2. Notification latency
	3. Total architecture cost throughout the stress test

Bonus:
- If your implementation can maintain a notification latency < 10s throughout the whole stress test.
## Architecture

To evaluate the architecture, during the demo session there will be a short discussion to understand the main design decisions taken by your team, followed by a qualitative assessment of the arcthitecture included in the report based on the principles of:
- Seperation of Concerns
- Scalability
## Report

10 page maximum report.
Sections: 
- Problem Description: A description of the main problems your team focused on solving when coming up with a possible architecture.
- Considerations and Trade-offs: The different solutions you considered to the problems you presented and their respective trade-offs.
- Implementation Architecture: A description of the final architecture implemented for the assignment.
- Future Work: What further improvements could be made to your architecture if your team had more time to dedicate to the project.

# Contact

If you have any questions regarding the assignment, feel free to message me on teams or sending me an email via `d.landau@uu.nl`.
