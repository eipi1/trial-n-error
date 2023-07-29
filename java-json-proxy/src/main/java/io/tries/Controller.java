package io.tries;

import java.net.URI;
import java.net.URISyntaxException;

import com.couchbase.client.java.Cluster;
import com.couchbase.client.java.ReactiveCluster;
import com.couchbase.client.java.ReactiveCollection;
import com.fasterxml.jackson.databind.node.ObjectNode;
import reactor.core.publisher.Flux;
import reactor.core.publisher.Mono;

import org.springframework.core.io.buffer.DataBuffer;
import org.springframework.core.io.buffer.DataBufferUtils;
import org.springframework.http.HttpStatus;
import org.springframework.http.MediaType;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RestController;
import org.springframework.web.reactive.function.client.WebClient;

@RestController
public class Controller {
	private final ReactiveCollection bucket;

	WebClient client;

	URI uri;

	public Controller() {
		client = WebClient.builder().build();
		try {
			uri = new URI("http://localhost:1080/hello");
		}
		catch (URISyntaxException e) {
			throw new RuntimeException(e);
		}
		Cluster cluster = Cluster.connect("127.0.0.1", "Administrator", "Administrator");
		ReactiveCluster reactiveCluster = cluster.reactive();
		bucket = reactiveCluster.bucket("test").defaultCollection();
	}

	@GetMapping("/jackson")
	public Mono<ObjectNode> get() {
		return client.get()
				.uri(uri)
				.exchangeToMono(clientResponse -> clientResponse.bodyToMono(ObjectNode.class));
	}

	@GetMapping("/json-no-parse")
	public ResponseEntity<Mono<DataBuffer>> getNoParse() {
		Mono<DataBuffer> dataBufferMono = client.get()
				.uri(uri)
				.exchangeToMono(clientResponse -> clientResponse.body((inputMessage, context) -> {
					Flux<DataBuffer> body = inputMessage.getBody();
					return DataBufferUtils.join(body);
				}));
		return ResponseEntity.status(HttpStatus.OK).contentType(MediaType.APPLICATION_JSON).body(dataBufferMono);
	}

	@GetMapping("/cb-no-parse")
	public ResponseEntity<Mono<byte[]>> getNoParseCouchbase() {
		Mono<byte[]> map = bucket.get("sample-json")
				.map(getResult -> getResult.contentAsBytes());
		return ResponseEntity.status(HttpStatus.OK).contentType(MediaType.APPLICATION_JSON).body(map);
	}
}
