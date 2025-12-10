from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
import sys
from io import StringIO
import traceback
import pandas as pd
import numpy as np

app = FastAPI()

class CodeRequest(BaseModel):
    code: str

@app.post("/execute")
async def execute_code(request: CodeRequest):
    """
    Executes Python code in a sandbox and returns the output.
    """
    # Create a buffer to capture print() output
    output_buffer = StringIO()
    sys.stdout = output_buffer

    # Context for the code execution (available libraries)
    # We give it pandas and numpy so it can do data analysis
    exec_globals = {
        "pd": pd,
        "np": np,
        "math": __import__("math")
    }

    try:
        # DANGEROUS: In a real startup, you'd use gVisor or Firecracker here.
        # For this MVP, Docker isolation is 'good enough'.
        exec(request.code, exec_globals)
        
        # Reset stdout
        sys.stdout = sys.__stdout__
        
        result = output_buffer.getvalue()
        if not result:
            result = "[Code executed successfully, but no output printed]"
        
        return {"status": "success", "output": result}

    except Exception:
        sys.stdout = sys.__stdout__
        error_msg = traceback.format_exc()
        return {"status": "error", "error": error_msg}

@app.get("/health")
async def health():
    return {"status": "ready"}