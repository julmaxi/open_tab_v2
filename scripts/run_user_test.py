import sqlite3
import base64
import threading
import multiprocessing
import sys
import argparse
from user_simulator import run_user_test, UserInfo, simulate_user_step
import random
from dataclasses import dataclass, field
from typing import List, Dict, Any

from selenium import webdriver
import selenium.webdriver.remote.webelement
from selenium.webdriver.chrome.options import Options
from selenium.webdriver.chrome.service import Service
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait, Select
from selenium.webdriver.support import expected_conditions as EC
from selenium.common.exceptions import (
    TimeoutException, 
    ElementNotInteractableException, 
    StaleElementReferenceException,
    NoSuchElementException
)


@dataclass
class State:
    user_info: UserInfo
    cookies: List[Dict[str, Any]] = field(default_factory=list)
    current_url: str = ""


def worker(queue, use_multiprocessing):
    options = Options()
    options.add_argument("--headless=new")
    driver = webdriver.Chrome(options=options)
    
    while True:
        state = queue.get()
        if state is None:
            break
        
        for cookie in state.cookies:
            driver.add_cookie(cookie)
        driver.get(state.current_url)
        
        user = state.user_info
        for _ in range(10):  # Run a few steps
            try:
                simulate_user_step(user, driver)
            except Exception as e:
                pass
        
        state.cookies = driver.get_cookies()
        state.current_url = driver.current_url
        queue.put(state)
    
    driver.quit()


def generate_registration_secret(uuid: str, key: str) -> str:
    registration_secret = bytearray(48)
    registration_secret[0:16] = uuid
    registration_secret[16:48] = key
    return base64.urlsafe_b64encode(registration_secret).rstrip(b'=').decode('utf-8')

def run_tests(server_base_url: str, database_path: str, tournament_id: str, use_multiprocessing: bool):
    conn = sqlite3.connect(database_path)
    cursor = conn.cursor()
    
    #cursor.execute("SELECT uuid, registration_key FROM participant WHERE tournament_id = ?", (tournament_id,))
    cursor.execute("SELECT uuid, registration_key FROM participant")
    participants = cursor.fetchall()

    random.shuffle(participants)
    
    if use_multiprocessing:
        from multiprocessing import Queue, Process as Worker
    else:
        from queue import Queue
        from threading import Thread as Worker
    
    queue = Queue()
    
    for uuid, registration_key in participants:
        secret = generate_registration_secret(uuid, registration_key)
        register_url = f"{server_base_url}/register/{secret}"
        user_info = UserInfo(register_url=register_url)
        state = State(user_info=user_info, current_url=register_url)
        queue.put(state)
    
    workers = []
    for _ in range(10):  # Number of workers
        worker_process = Worker(target=worker, args=(queue, use_multiprocessing))
        workers.append(worker_process)
        worker_process.start()

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run user tests for a tournament.")
    parser.add_argument("server_base_url", type=str, help="The base URL of the server.")
    parser.add_argument("database_path", type=str, help="The path to the SQLite database.")
    parser.add_argument("tournament_id", type=str, help="The ID of the tournament.")
    parser.add_argument("--multiprocessing", action="store_true", help="Use multiprocessing instead of threading.")
    
    args = parser.parse_args()
    
    run_tests(args.server_base_url, args.database_path, args.tournament_id, args.multiprocessing)
