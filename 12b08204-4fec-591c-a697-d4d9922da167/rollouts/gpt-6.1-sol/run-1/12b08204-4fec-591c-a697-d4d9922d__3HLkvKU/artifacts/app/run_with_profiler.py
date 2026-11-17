"""Run the development ASGI server."""
import uvicorn

if __name__ == "__main__":
    uvicorn.run("argus_backend:create_app", factory=True)
