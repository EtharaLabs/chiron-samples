package me.zodac.folding.rest.response;

import static me.zodac.folding.rest.util.RestUtilConstants.GSON;
import java.util.Collection;
import javax.servlet.http.HttpServletRequest;
import org.springframework.http.ResponseEntity;
import org.springframework.web.util.UriComponentsBuilder;

/** HTTP responses shared by the controllers, including conditional resource reads. */
public final class Responses {
    private Responses() {}
    public static ResponseEntity<String> ok() { return ResponseEntity.ok().build(); }
    public static ResponseEntity<String> ok(Object entity) { return json(200, entity); }
    public static ResponseEntity<String> ok(Collection<?> entities) {
        return ResponseEntity.ok().header("X-Total-Count", String.valueOf(entities.size())).body(GSON.toJson(entities));
    }
    public static ResponseEntity<String> ok(Object entity, UriComponentsBuilder location) { return json(200, entity); }
    public static ResponseEntity<String> created(Object entity, UriComponentsBuilder location) {
        return ResponseEntity.created(location.build().toUri()).body(GSON.toJson(entity));
    }
    public static ResponseEntity<String> cachedOk(Object entity, HttpServletRequest request, int seconds) {
        return cached(entity, entity.hashCode(), request, seconds);
    }
    public static ResponseEntity<String> cachedOk(Collection<?> entities, HttpServletRequest request, int seconds) {
        return cached(entities, entities.stream().mapToInt(Object::hashCode).sum(), request, seconds);
    }
    private static ResponseEntity<String> cached(Object entity, int hash, HttpServletRequest request, int seconds) {
        String tag = "\"" + hash + "\"";
        String match = request.getHeader("If-Match");
        String none = request.getHeader("If-None-Match");
        int status = 200;
        if (match != null && !matches(match, tag, false)) status = 412;
        else if (none != null && matches(none, tag, false)) status = 304;
        ResponseEntity.BodyBuilder builder = ResponseEntity.status(status).header("Cache-Control", "no-transform, max-age=" + seconds);
        builder.eTag(tag);
        if (status != 200) return builder.build();
        if (entity instanceof Collection<?>) builder.header("X-Total-Count", String.valueOf(((Collection<?>) entity).size()));
        return builder.body(GSON.toJson(entity));
    }
    private static boolean matches(String header, String tag, boolean weak) {
        for (String part : header.split(",")) {
            String value = part.trim();
            if (weak && value.startsWith("W/")) value = value.substring(2);
            if (value.equals("*") || value.equals(tag)) return true;
        }
        return false;
    }
    private static ResponseEntity<String> json(int status, Object entity) { return ResponseEntity.status(status).body(GSON.toJson(entity)); }
    public static ResponseEntity<String> badRequest(String message) { return json(400, ErrorResponse.create(message)); }
    public static ResponseEntity<String> badRequest(Object entity) { return json(400, entity); }
    public static ResponseEntity<String> nullRequest() { return badRequest("Payload is null"); }
    public static ResponseEntity<String> unauthorized() { return ResponseEntity.status(401).build(); }
    public static ResponseEntity<String> forbidden() { return ResponseEntity.status(403).build(); }
    public static ResponseEntity<String> notFound() { return ResponseEntity.status(404).build(); }
    public static ResponseEntity<String> conflict(Object entity) { return json(409, entity); }
    public static ResponseEntity<String> serverError() { return ResponseEntity.status(500).build(); }
    public static ResponseEntity<String> serviceUnavailable() { return ResponseEntity.status(503).build(); }
}
