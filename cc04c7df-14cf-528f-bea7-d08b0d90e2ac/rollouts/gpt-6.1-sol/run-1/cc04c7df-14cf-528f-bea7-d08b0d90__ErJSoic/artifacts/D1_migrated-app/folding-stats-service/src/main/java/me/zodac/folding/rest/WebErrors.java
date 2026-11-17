package me.zodac.folding.rest;

import javax.servlet.http.HttpServletResponse;
import org.springframework.http.ResponseEntity;
import org.springframework.http.converter.HttpMessageNotReadableException;
import org.springframework.web.HttpMediaTypeNotAcceptableException;
import org.springframework.web.HttpMediaTypeNotSupportedException;
import org.springframework.web.HttpRequestMethodNotSupportedException;
import org.springframework.web.bind.annotation.ExceptionHandler;
import org.springframework.web.bind.annotation.RestControllerAdvice;
import org.springframework.web.servlet.NoHandlerFoundException;

@RestControllerAdvice
public class WebErrors {
    private final String redirectUrl;
    public WebErrors(@org.springframework.beans.factory.annotation.Value("${REDIRECT_URL:https://zodac.me}") String redirectUrl) {
        this.redirectUrl = redirectUrl;
    }
    @ExceptionHandler(NoHandlerFoundException.class)
    public ResponseEntity<Void> missing(HttpServletResponse response) {
        return ResponseEntity.status(303).header("Location", redirectUrl).build();
    }
    @ExceptionHandler(HttpMessageNotReadableException.class)
    public ResponseEntity<String> unreadable(HttpServletResponse response) {
        return ResponseEntity.badRequest().header("Content-Type", "text/html;charset=UTF-8").body("<html><body>Bad Request</body></html>");
    }
    @ExceptionHandler({HttpMediaTypeNotAcceptableException.class, HttpMediaTypeNotSupportedException.class, HttpRequestMethodNotSupportedException.class})
    public ResponseEntity<Void> protocol(Exception error, HttpServletResponse response) {
        return ResponseEntity.status(error instanceof HttpMediaTypeNotAcceptableException ? 406 : error instanceof HttpMediaTypeNotSupportedException ? 415 : 405).build();
    }
    @ExceptionHandler(Exception.class)
    public ResponseEntity<String> failed(Exception error, HttpServletResponse response) {
        response.reset();
        return ResponseEntity.status(500).header("Content-Type", "text/html;charset=UTF-8").body("<html><body>Internal Server Error</body></html>");
    }
}
